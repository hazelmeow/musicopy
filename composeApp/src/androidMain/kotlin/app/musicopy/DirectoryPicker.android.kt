package app.musicopy

actual class DirectoryPicker {
    private var activity: MainActivity

    actual constructor(platformContext: PlatformContext) {
        this.activity = platformContext.mainActivity
    }

    actual suspend fun pickDownloadDirectory() {
        activity.observer.openDocumentTree.launch(null)
    }
}
