package zip.meows.musicopy

import androidx.compose.ui.platform.ClipEntry
import uniffi.musicopy.CoreOptions

expect class PlatformContext private constructor() {
    val name: String
}

expect fun toClipEntry(string: String): ClipEntry

interface ICoreProvider {
    fun getOptions(): CoreOptions {
        return CoreOptions(
            initLogging = true,
            inMemory = false
        )
    }
}

expect object CoreProvider : ICoreProvider;

expect fun formatFloat(f: Float, decimals: Int): String
