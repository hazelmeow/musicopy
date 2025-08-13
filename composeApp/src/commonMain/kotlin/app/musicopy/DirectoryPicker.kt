package app.musicopy

expect class DirectoryPicker {
    constructor(platformContext: PlatformContext)

    suspend fun pickDownloadDirectory()
}
