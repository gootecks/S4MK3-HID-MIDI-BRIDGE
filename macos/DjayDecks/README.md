# DjayDecks

Reads djay Pro's decks for the S8 deck page (#45): Accessibility state per
deck plus per-track analysis from djay's metadata cache. It never sends
anything to djay or the S8. Details, snapshot fields and limits:
[`docs/s8/deck-pages.md`](../../docs/s8/deck-pages.md).

## Origin and files

Ported from the MIT-licensed `djay-phrase-tool` prototype (`Sources/DjayBridge`),
which builds on [kyleawayan/djay-pro-bridge](https://github.com/kyleawayan/djay-pro-bridge).
The copyright and permission notice is in [`NOTICE`](NOTICE).

| File | Status |
| ---- | ------ |
| `DeckReading.swift` | Ported (`DeckParser`, label parsing incl. French labels); `DeckReading` fields renamed |
| `DeckAccessibility.swift` | Ported (AX tree walk) behind the `DeckAccessibility` protocol, changed to one walk for all decks; `DeckReader` and `DeckAccessError` are new |
| `TimeInterpolator.swift` | Ported; clock is injectable; `playingSince` added |
| `PlayStateDebouncer.swift` | Ported; clock is injectable; `activeSince` and the sample gap limit added |
| `TrackCatalog.swift` | Adapted from `DjayLibraryLookup` (title and artist lookup); `AnalysisStore` and `LibraryCatalog` are new and use `DjayLibrary` |
| `TrackAnalysis.swift`, `Inflate.swift` | New (decoding of the `.djayMetadata` plist) |
| `DeckTracker.swift`, `DeckMonitor.swift` | New (per-deck state, polling) |

Not ported: phrase counting, downbeat stores, the prototype UI.

## Dependencies

`DjayLibrary` (read-only database snapshot, track and cue lookup) and
`S8Views` (`DeckSnapshot`, so a deck's state maps straight to the page).
`S8Views` stays free of Accessibility and file access; `DjayDecks` is the
only place that touches djay.
