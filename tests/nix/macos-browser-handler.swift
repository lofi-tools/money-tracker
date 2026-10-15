// Acceptance helper: snapshot/restore the user's URL defaults around a test App.
import AppKit
import CoreServices
import Foundation
let args = Array(CommandLine.arguments.dropFirst())
func fail(_ message: String) -> Never { fputs(message + "\n", stderr); exit(1) }
guard let action = args.first else { fail("get | set-path SCHEME APP | register APP | resolve URL") }
switch action {
case "get":
    var result: [String: String] = [:]
    for scheme in ["http", "https"] {
        if let value = LSCopyDefaultHandlerForURLScheme(scheme as CFString)?.takeRetainedValue() {
            result[scheme] = value as String
        }
    }
    print(String(data: try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys]), encoding: .utf8)!)
case "set-path":
    guard args.count == 3 else { fail("set-path SCHEME APP") }
    var completed = false
    var failure: Error? = nil
    NSWorkspace.shared.setDefaultApplication(at: URL(fileURLWithPath: args[2]), toOpenURLsWithScheme: args[1]) { error in
        failure = error
        completed = true
    }
    let deadline = Date().addingTimeInterval(20)
    while !completed && Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.1)) }
    guard completed else { fail("Default handler approval timed out") }
    if let error = failure { fail("set-path failed: \(error)") }
case "register":
    guard args.count == 2 else { fail("register APP") }
    let status = LSRegisterURL(URL(fileURLWithPath: args[1]) as CFURL, true)
    guard status == noErr else { fail("LSRegisterURL failed: \(status)") }
case "resolve":
    guard args.count == 2, let url = URL(string: args[1]) else { fail("resolve URL") }
    guard let app = NSWorkspace.shared.urlForApplication(toOpen: url) else { fail("No URL handler") }
    print(app.path)
default: fail("Unknown action")
}
