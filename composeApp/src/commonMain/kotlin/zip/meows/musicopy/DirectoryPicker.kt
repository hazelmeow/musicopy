package zip.meows.musicopy

import androidx.compose.runtime.Composable

expect class DirectoryPicker {
    companion object {
        @Composable
        fun get(): DirectoryPicker
    }

    fun start()
}
