package zip.meows.musicopy

expect class DirectoryPicker {
    constructor(platformContext: PlatformContext)

    fun pickDownloadDirectory()
}
