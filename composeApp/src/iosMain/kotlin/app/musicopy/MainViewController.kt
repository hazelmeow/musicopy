package app.musicopy

import androidx.compose.ui.window.ComposeUIViewController

fun MainViewController() {
    val platformAppContext = PlatformAppContext()
    val platformActivityContext = PlatformActivityContext()

    ComposeUIViewController {
        App(
            platformAppContext = platformAppContext,
            platformActivityContext = platformActivityContext,
            coreInstance = TODO()
        )
    }
}
