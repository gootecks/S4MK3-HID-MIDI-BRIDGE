@testable import S8App
import XCTest

final class MappingInstallerTests: XCTestCase {
    func testNewestRevisionIsChosenNumerically() throws {
        let folder = TestFolder()
        defer { folder.remove() }
        let source = folder.folder("bundle")
        for name in ["Kontrol S8 Bridge r9", "Kontrol S8 Bridge r10", "Kontrol S8 Bridge Evan r4"] {
            let file = source.appendingPathComponent("\(name).djayMidiMapping")
            FileManager.default.createFile(atPath: file.path, contents: nil)
        }
        let installer = MappingInstaller(source: source, destination: folder.folder("djay"))
        XCTAssertEqual(try installer.mappingName(for: .standard), "Kontrol S8 Bridge r10")
        XCTAssertEqual(try installer.mappingName(for: .evan), "Kontrol S8 Bridge Evan r4")
    }

    func testInstallNeverOverwrites() throws {
        let folder = TestFolder()
        defer { folder.remove() }
        let source = folder.folder("bundle")
        let djay = folder.folder("djay")
        let name = "Kontrol S8 Bridge r9.djayMidiMapping"
        FileManager.default.createFile(atPath: source.appendingPathComponent(name).path, contents: Data("new".utf8))
        FileManager.default.createFile(atPath: djay.appendingPathComponent(name).path, contents: Data("edited".utf8))
        let installer = MappingInstaller(source: source, destination: djay)
        XCTAssertEqual(try installer.installMissing(), [])
        XCTAssertEqual(try Data(contentsOf: djay.appendingPathComponent(name)), Data("edited".utf8))
    }

    func testMissingDjayFolderIsReported() {
        let folder = TestFolder()
        defer { folder.remove() }
        let missing = folder.url.appendingPathComponent("none")
        let installer = MappingInstaller(source: folder.folder("bundle"), destination: missing)
        XCTAssertThrowsError(try installer.installMissing()) { error in
            XCTAssertEqual(error as? MappingError, .djayFolderMissing(missing.path))
        }
    }
}
