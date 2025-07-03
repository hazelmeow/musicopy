package zip.meows.musicopy

actual class DirectoryPicker {
    private var activity: MainActivity

    actual constructor(platformContext: PlatformContext) {
        this.activity = platformContext.mainActivity
    }

    actual fun pickDownloadDirectory() {
        activity.observer.openDocumentTree.launch(null)
    }
}
