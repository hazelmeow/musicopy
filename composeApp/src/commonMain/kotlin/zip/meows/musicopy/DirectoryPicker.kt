package zip.meows.musicopy

expect class DirectoryPicker {
    constructor(platformContext: PlatformContext)

    suspend fun pickDownloadDirectory()
}
