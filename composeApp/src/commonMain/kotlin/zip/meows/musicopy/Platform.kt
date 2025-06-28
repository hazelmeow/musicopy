package zip.meows.musicopy

import androidx.compose.ui.platform.ClipEntry

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform

expect fun toClipEntry(string: String): ClipEntry
