package zip.meows.musicopy

import androidx.compose.runtime.Composable

actual class DirectoryPicker() {
    actual companion object {
        @Composable
        actual fun get(): DirectoryPicker {
            return DirectoryPicker()
        }
    }

    actual fun start() {
        TODO()
    }
}
