package app.musicopy

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.window.ComposeUIViewController
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import platform.UIKit.UIViewController

fun MainViewController(): UIViewController {
    val platformAppContext = PlatformAppContext()
    val platformActivityContext = PlatformActivityContext()

    return ComposeUIViewController {
        var coreInstance: CoreInstance? by remember { mutableStateOf(null) }
        GlobalScope.launch {
            coreInstance = CoreInstance.start(platformAppContext)
        }

        coreInstance?.let { coreInstance ->
            App(
                platformAppContext = platformAppContext,
                platformActivityContext = platformActivityContext,
                coreInstance = coreInstance
            )
        }
    }
}
