//! Screen transport ownership, rendering and release run only on this worker.

use super::*;
use std::time::{Duration, Instant};

fn emit<S: Fn(u64, State)>(
    shared: &Shared,
    status: Option<&S>,
    generation: u64,
    id: u64,
    state: State,
) {
    if let Some(status) = status
        && shared.0.lock().is_ok_and(|mailbox| {
            mailbox
                .latest
                .is_some_and(|request| request.generation == generation)
        })
    {
        status(id, state);
    }
}

fn await_work(
    shared: &Shared,
    current: &mut Option<[page::Page; 2]>,
    enabled: bool,
    next_flash: Option<Instant>,
    handled_generation: u64,
) -> anyhow::Result<(Option<Request>, bool, bool)> {
    let mut mailbox = shared
        .0
        .lock()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    while mailbox.pending.is_none()
        && mailbox
            .latest
            .is_none_or(|request| request.generation == handled_generation)
        && !mailbox.release_required
        && !mailbox.stop
        && (!enabled || next_flash.is_none())
    {
        mailbox = shared
            .1
            .wait(mailbox)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
    }
    while mailbox.pending.is_none()
        && mailbox
            .latest
            .is_none_or(|request| request.generation == handled_generation)
        && !mailbox.release_required
        && !mailbox.stop
        && enabled
        && next_flash.is_some_and(|deadline| Instant::now() < deadline)
    {
        let remaining = next_flash
            .unwrap_or_else(Instant::now)
            .saturating_duration_since(Instant::now());
        mailbox = shared
            .1
            .wait_timeout(mailbox, remaining)
            .map_err(|error| anyhow::anyhow!("{error}"))?
            .0;
    }
    *current = mailbox.pending.take().or(*current);
    let request = mailbox
        .latest
        .filter(|request| request.generation != handled_generation);
    let release = mailbox.release_required;
    mailbox.release_required = false;
    Ok((request, release, mailbox.stop))
}

fn interrupted(shared: &Shared, generation: u64) -> anyhow::Result<bool> {
    let mailbox = shared
        .0
        .lock()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(mailbox.stop
        || mailbox
            .latest
            .is_some_and(|request| request.generation != generation))
}

fn render_updates<W>(
    shared: &Shared,
    write: &mut W,
    frame: &mut Frame,
    pages: [page::Page; 2],
    previous: Option<[(page::Page, bool); 2]>,
    rendered: [(page::Page, bool); 2],
    generation: u64,
) -> anyhow::Result<bool>
where
    W: FnMut(Display, &Frame) -> anyhow::Result<()>,
{
    for (side, screen) in [Display::Left, Display::Right].into_iter().enumerate() {
        if interrupted(shared, generation)? {
            return Ok(false);
        }
        if previous
            .as_ref()
            .is_none_or(|old| old[side] != rendered[side])
        {
            page::render_phase(&pages[side], frame, rendered[side].1);
            write(screen, frame)?;
        }
    }
    Ok(!interrupted(shared, generation)?)
}

fn release_transport<W>(
    transport: &mut Option<W>,
    frame: &mut Frame,
    stopping: bool,
) -> anyhow::Result<()>
where
    W: FnMut(Display, &Frame) -> anyhow::Result<()>,
{
    if let Some(mut write) = transport.take() {
        if stopping {
            page::stopped(frame);
        } else {
            page::render(&page::Page::Empty, frame);
        }
        let result = write(Display::Left, frame).and_then(|()| write(Display::Right, frame));
        drop(write);
        result?;
    }
    Ok(())
}

fn advance_flash(pages: &[page::Page; 2], bright: &mut bool, next_flash: &mut Option<Instant>) {
    let now = Instant::now();
    if pages.iter().any(page::Page::selecting) {
        if next_flash.is_none() {
            *bright = true;
            *next_flash = Some(now + Duration::from_millis(500));
        } else if next_flash.is_some_and(|deadline| now >= deadline) {
            *bright = !*bright;
            *next_flash = Some(now + Duration::from_millis(500));
        }
    } else {
        *bright = true;
        *next_flash = None;
    }
}

pub(super) fn run<F, W, S>(
    shared: &Shared,
    mut open: F,
    mut enabled: bool,
    status: Option<&S>,
) -> anyhow::Result<()>
where
    F: FnMut() -> anyhow::Result<W>,
    W: FnMut(Display, &Frame) -> anyhow::Result<()>,
    S: Fn(u64, State),
{
    let mut transport: Option<W> = None;
    let mut frame = Frame::default();
    let mut current = None;
    let mut previous = None;
    let mut bright = true;
    let mut next_flash: Option<Instant> = None;
    let mut handled_generation = 0;
    let mut owner = (0, 0);
    let mut active = false;
    loop {
        let (request, release, stopping) = await_work(
            shared,
            &mut current,
            enabled,
            next_flash,
            handled_generation,
        )?;
        if stopping {
            release_transport(&mut transport, &mut frame, true)?;
            return Ok(());
        }
        if let Some(request) = request {
            handled_generation = request.generation;
            owner = (request.generation, request.id);
            enabled = matches!(request.action, Action::On | Action::Retry);
            active = false;
            if enabled {
                emit(shared, status, owner.0, owner.1, State::Pending);
            }
        }
        if release || !enabled {
            if transport.is_some() {
                let clear = release_transport(&mut transport, &mut frame, false);
                previous = None;
                next_flash = None;
                if let Err(error) = clear {
                    tracing::warn!(%error, "SCREENS_DISABLED: screen clear failed");
                    emit(shared, status, owner.0, owner.1, State::Failed);
                    enabled = false;
                    continue;
                }
            }
            if !enabled {
                emit(shared, status, owner.0, owner.1, State::Inactive);
                continue;
            }
        }
        let Some(pages) = current else { continue };
        if !enabled {
            continue;
        }
        if transport.is_none() {
            match open() {
                Ok(write) => transport = Some(write),
                Err(error) => {
                    tracing::warn!(%error, "SCREENS_DISABLED: screen open failed");
                    emit(shared, status, owner.0, owner.1, State::Failed);
                    enabled = false;
                    continue;
                }
            }
        }
        advance_flash(&pages, &mut bright, &mut next_flash);
        let rendered = pages.map(|page| (page, !page.selecting() || bright));
        let outcome = render_updates(
            shared,
            transport.as_mut().context("screen transport missing")?,
            &mut frame,
            pages,
            previous,
            rendered,
            handled_generation,
        );
        match outcome {
            Err(error) => {
                drop(transport.take());
                tracing::warn!(%error, "SCREENS_DISABLED: screen write failed");
                emit(shared, status, owner.0, owner.1, State::Failed);
                enabled = false;
                previous = None;
                next_flash = None;
            }
            Ok(false) => {}
            Ok(true) => {
                previous = Some(rendered);
                if !active {
                    active = true;
                    emit(shared, status, owner.0, owner.1, State::Active);
                }
            }
        }
    }
}
