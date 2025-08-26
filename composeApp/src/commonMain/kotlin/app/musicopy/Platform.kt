package app.musicopy

import androidx.compose.ui.platform.ClipEntry
import uniffi.musicopy.CoreOptions

/**
 * Platform-specific application/process-scoped context.
 */
expect class PlatformAppContext private constructor() {
    val name: String
}

/**
 * Platform-specific activity/scene-scoped context.
 */
expect class PlatformActivityContext private constructor() {}

interface ICoreProvider {
    fun getOptions(platformAppContext: PlatformAppContext): CoreOptions {
        return CoreOptions(
            initLogging = true,
            inMemory = false,
            projectDirs = null,
        )
    }
}

expect object CoreProvider : ICoreProvider;

expect fun toClipEntry(string: String): ClipEntry

expect fun formatFloat(f: Float, decimals: Int): String
