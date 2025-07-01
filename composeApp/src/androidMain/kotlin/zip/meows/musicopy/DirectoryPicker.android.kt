package zip.meows.musicopy

import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.Composable

actual class DirectoryPicker() {
    lateinit var activity: MainActivity
        private set

    constructor(activity: MainActivity) : this() {
        this.activity = activity
    }

    actual companion object {
        @Composable
        actual fun get(): DirectoryPicker {
            val activity = LocalActivity.current
            check(activity != null)
            check(activity is MainActivity)

            return DirectoryPicker(activity)
        }
    }

    actual fun start() {
        activity.observer.openDocumentTree.launch(null)
    }
}
