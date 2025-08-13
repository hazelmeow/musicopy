package app.musicopy

import androidx.compose.ui.window.ComposeUIViewController

fun MainViewController() {
    val platformContext = PlatformContext()

    ComposeUIViewController {
        App(platformContext)
    }
}
