import DjayDecks
import DjaySidebar
import Foundation
import S8Control
import S8Session
import S8Views

extension DjaySidebar: SidebarControlling {}

extension PanelSession {
    static func live(bridge: Bridge, options: HostOptions) -> PanelSession {
        let sidebar = DjaySidebar(root: LiveRoot())
        let router = ControlRouter(sidebar: sidebar)
        let monitor = options.deck.map { DeckMonitor.live(decks: [$0]) }
        let sources = Sources(
            route: { router.handle($0) },
            readSidebar: { sidebar.snapshot() },
            pollDecks: monitor.map { monitor in { monitor.poll() } },
            deckSnapshot: { options.deck.flatMap { monitor?.snapshot(deck: $0) } },
            send: { bridge.send($0) },
            log: options.logDeckPolls ? report : nil
        )
        return PanelSession(sources: sources, screens: options.screens, sidebarInterval: options.poll)
    }
}
