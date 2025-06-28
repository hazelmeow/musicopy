package zip.meows.musicopy

import androidx.compose.ui.platform.ClipEntry
import java.awt.datatransfer.StringSelection

class JVMPlatform: Platform {
    override val name: String = "Java ${System.getProperty("java.version")}"
}

actual fun getPlatform(): Platform = JVMPlatform()

actual fun toClipEntry(string: String): ClipEntry = ClipEntry(StringSelection(string))
