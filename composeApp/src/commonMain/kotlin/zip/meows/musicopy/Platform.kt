package zip.meows.musicopy

import androidx.compose.ui.platform.ClipEntry

expect class PlatformContext private constructor() {
    val name: String
}

expect fun toClipEntry(string: String): ClipEntry
