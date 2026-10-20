import AppKit
import Sparkle

/// Installs the latest release when the library's "install" is clicked (`update-check`): Sparkle fetches the appcast,
/// downloads the release's disk image, verifies its EdDSA signature, replaces the app and relaunches it, with every
/// question answered "install" and no window of its own. Any failure leaves the line offering "download" for the rest of
/// the run. Sparkle is made on the first click and never checks by itself (`SUEnableAutomaticChecks` is off).
final class Installer: NSObject, SPUUserDriver {
    static let shared = Installer()

    enum Phase { case offered, installing, failed }
    private(set) var phase = Phase.offered
    /// The library's line, told on the main thread when the phase changes.
    var onChange: (() -> Void)?
    private var updater: SPUUpdater?

    func install() {
        guard phase == .offered else { return }
        if updater == nil {
            let updater = SPUUpdater(hostBundle: .main, applicationBundle: .main, userDriver: self, delegate: nil)
            // Sparkle's own would carry the app's and Sparkle's versions.
            updater.userAgentString = "Forzac"
            do { try updater.start() } catch { return set(.failed) }
            self.updater = updater
        }
        set(.installing)
        updater?.checkForUpdates()
    }

    private func set(_ phase: Phase) {
        self.phase = phase
        onChange?()
    }

    // MARK: SPUUserDriver

    func show(_ request: SPUUpdatePermissionRequest, reply: @escaping (SUUpdatePermissionResponse) -> Void) {
        reply(SUUpdatePermissionResponse(automaticUpdateChecks: false, sendSystemProfile: false))
    }

    func showUpdateFound(with appcastItem: SUAppcastItem, state: SPUUserUpdateState, reply: @escaping (SPUUserUpdateChoice) -> Void) {
        reply(.install)
    }

    func showReady(toInstallAndRelaunch reply: @escaping (SPUUserUpdateChoice) -> Void) {
        reply(.install)
    }

    func showUpdateNotFoundWithError(_ error: any Error, acknowledgement: @escaping () -> Void) {
        acknowledgement()
        set(.failed)
    }

    func showUpdaterError(_ error: any Error, acknowledgement: @escaping () -> Void) {
        acknowledgement()
        set(.failed)
    }

    func showUserInitiatedUpdateCheck(cancellation: @escaping () -> Void) {}
    func showUpdateReleaseNotes(with downloadData: SPUDownloadData) {}
    func showUpdateReleaseNotesFailedToDownloadWithError(_ error: any Error) {}
    func showDownloadInitiated(cancellation: @escaping () -> Void) {}
    func showDownloadDidReceiveExpectedContentLength(_ expectedContentLength: UInt64) {}
    func showDownloadDidReceiveData(ofLength length: UInt64) {}
    func showDownloadDidStartExtractingUpdate() {}
    func showExtractionReceivedProgress(_ progress: Double) {}
    func showInstallingUpdate(withApplicationTerminated applicationTerminated: Bool, retryTerminatingApplication: @escaping () -> Void) {}
    func showUpdateInstalledAndRelaunched(_ relaunched: Bool, acknowledgement: @escaping () -> Void) { acknowledgement() }
    func showUpdateInFocus() {}
    func dismissUpdateInstallation() {}
}
