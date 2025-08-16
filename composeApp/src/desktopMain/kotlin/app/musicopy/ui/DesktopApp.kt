package app.musicopy.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.MutableCreationExtras
import androidx.lifecycle.viewmodel.compose.viewModel
import app.musicopy.CoreViewModel
import app.musicopy.PlatformContext

@Composable
fun DesktopApp(
    platformContext: PlatformContext,
) {
    val extras = MutableCreationExtras().apply {
        set(CoreViewModel.PLATFORM_CONTEXT_KEY, platformContext)
    }
    val viewModel: CoreViewModel = viewModel(factory = CoreViewModel.Factory, extras = extras)

    val model by viewModel.state.collectAsState()

    Theme {
        Box(modifier = Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.background)) {
            model?.let {
                DesktopHome(
                    model = it,
                    showHints = true,
                    onAcceptAndTrust = { nodeId ->
                        viewModel.instance.acceptConnectionAndTrust(
                            nodeId
                        )
                    },
                    onAcceptOnce = { nodeId -> viewModel.instance.acceptConnection(nodeId) },
                    onDeny = { nodeId -> viewModel.instance.denyConnection(nodeId) },
                    onAddLibraryRoot = { name, path ->
                        viewModel.instance.addLibraryRoot(
                            name,
                            path
                        )
                    },
                    onRemoveLibraryRoot = { name -> viewModel.instance.removeLibraryRoot(name) },
                    onRescanLibrary = { viewModel.instance.rescanLibrary() }
                )
            }
        }
    }
}
