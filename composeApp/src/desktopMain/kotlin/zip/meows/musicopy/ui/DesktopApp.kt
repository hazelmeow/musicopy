package zip.meows.musicopy.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.viewmodel.MutableCreationExtras
import androidx.lifecycle.viewmodel.compose.viewModel
import zip.meows.musicopy.CoreViewModel
import zip.meows.musicopy.PlatformContext

@Composable
fun DesktopApp(
    platformContext: PlatformContext,
) {
    val extras = MutableCreationExtras().apply {
        set(CoreViewModel.PLATFORM_CONTEXT_KEY, platformContext)
    }
    val viewModel: CoreViewModel = viewModel(factory = CoreViewModel.Factory, extras = extras)
    
    val model by viewModel.state.collectAsState()

    MaterialTheme {
        model?.let {
            DesktopHome(
                model = it,
                onAcceptAndTrust = { nodeId -> viewModel.instance.acceptConnection(nodeId) }, // TODO
                onAcceptOnce = { nodeId -> viewModel.instance.acceptConnection(nodeId) },
                onDeny = { nodeId -> viewModel.instance.denyConnection(nodeId) },
                onAddLibraryRoot = { name, path -> viewModel.instance.addLibraryRoot(name, path) },
                onRemoveLibraryRoot = { name -> viewModel.instance.removeLibraryRoot(name) },
                onRescanLibrary = { viewModel.instance.rescanLibrary() }
            )
        }
    }
}
