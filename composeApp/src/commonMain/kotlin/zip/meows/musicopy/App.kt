package zip.meows.musicopy

import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.MutableCreationExtras
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.musicopy.CoreException
import zip.meows.musicopy.ui.screens.ConnectManually
import zip.meows.musicopy.ui.screens.ConnectManuallyScreen
import zip.meows.musicopy.ui.screens.ConnectQR
import zip.meows.musicopy.ui.screens.ConnectQRScreen
import zip.meows.musicopy.ui.screens.Home
import zip.meows.musicopy.ui.screens.HomeScreen
import zip.meows.musicopy.ui.screens.PreTransfer
import zip.meows.musicopy.ui.screens.PreTransferScreen
import zip.meows.musicopy.ui.screens.Transfer
import zip.meows.musicopy.ui.screens.TransferScreen
import zip.meows.musicopy.ui.screens.Waiting
import zip.meows.musicopy.ui.screens.WaitingScreen

@Composable
fun App(
    platformContext: PlatformContext,
    navController: NavHostController = rememberNavController(),
) {
    val extras = MutableCreationExtras().apply {
        set(CoreViewModel.PLATFORM_CONTEXT_KEY, platformContext)
    }
    val viewModel: CoreViewModel = viewModel(factory = CoreViewModel.Factory, extras = extras)

    val model by viewModel.state.collectAsState()

    val directoryPicker = remember { DirectoryPicker(platformContext) }

    val scope = rememberCoroutineScope()
    var connectCount by remember { mutableStateOf(0) }
    val isConnecting = connectCount > 0

    val onConnect = { nodeId: String ->
        scope.launch {
            connectCount += 1
            try {
                viewModel.instance.connect(nodeId = nodeId)
                delay(100) // TODO
                val client = viewModel.state.value?.node?.clients?.find { it.nodeId == nodeId }
                if (client?.accepted == true) {
                    navController.navigate(PreTransfer(nodeId = nodeId))
                } else {
                    navController.navigate(Waiting(nodeId = nodeId))
                }
            } catch (e: CoreException) {
                // TODO
                println("error during connect: $e")
            } finally {
                connectCount -= 1
            }
        }
        Unit
    }

    MaterialTheme {
        NavHost(
            navController = navController,
            startDestination = Home,
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
            enterTransition = {
                slideIntoContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Left,
                    animationSpec = tween(700)
                )
            },
            exitTransition = {
                slideOutOfContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Left,
                    animationSpec = tween(700)
                )
            },
            popEnterTransition = {
                slideIntoContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Right,
                    animationSpec = tween(700)
                )
            },
            popExitTransition = {
                slideOutOfContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Right,
                    animationSpec = tween(700)
                )
            }
        ) {
            composable<Home> {
                // TODO: make this better...
                model?.let {
                    HomeScreen(
                        model = it,
                        onPickDownloadDirectory = {
                            scope.launch {
                                directoryPicker.pickDownloadDirectory()
                            }
                        },
                        onConnectQRButtonClicked = { navController.navigate(ConnectQR) },
                        onConnectManuallyButtonClicked = {
                            navController.navigate(
                                ConnectManually
                            )
                        }
                    )
                }
            }
            composable<ConnectQR> {
                model?.let {
                    ConnectQRScreen(
                        isConnecting = isConnecting,
                        onSubmit = onConnect,
                        onCancel = {
                            navController.popBackStack(Home, inclusive = false)
                        }
                    )
                }
            }
            composable<ConnectManually> {
                model?.let {
                    ConnectManuallyScreen(
                        isConnecting = isConnecting,
                        onSubmit = onConnect,
                        onCancel = {
                            navController.popBackStack(Home, inclusive = false)
                        }
                    )
                }
            }
            composable<Waiting> { backStackEntry ->
                val waiting: Waiting = backStackEntry.toRoute()
                val nodeId = waiting.nodeId
                model?.let { model ->
                    val client = model.node.clients.find { x -> x.nodeId == nodeId }

                    if (client?.accepted == true && navController.currentDestination?.hasRoute<Waiting>() == true) {
                        navController.navigate(PreTransfer(nodeId = nodeId))
                    }

                    client?.let { clientModel ->
                        WaitingScreen(
                            clientModel = clientModel,
                            onCancel = {
                                navController.popBackStack(Home, inclusive = false)
                            }
                        )
                    }
                }
            }
            composable<PreTransfer> { backStackEntry ->
                val preTransfer: PreTransfer = backStackEntry.toRoute()
                val nodeId = preTransfer.nodeId
                model?.let { model ->
                    val client = model.node.clients.find { x -> x.nodeId == nodeId }

                    val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(
                        null
                    )

                    client?.let { clientModel ->
                        PreTransferScreen(
                            clientModel = clientModel,
                            onDownloadAll = {
                                downloadDirectory?.let { downloadDirectory ->
                                    viewModel.instance.downloadAll(nodeId, downloadDirectory)
                                    navController.navigate(Transfer(nodeId = nodeId))
                                } ?: run {
                                    // TODO toast?
                                    println("download directory is null")
                                }
                            },
                            onCancel = {
                                navController.popBackStack(Home, inclusive = false)
                            }
                        )
                    }
                }
            }
            composable<Transfer> { backStackEntry ->
                val transfer: Transfer = backStackEntry.toRoute()
                val nodeId = transfer.nodeId
                model?.let { model ->
                    val client = model.node.clients.find { x -> x.nodeId == nodeId }

                    client?.let { clientModel ->
                        TransferScreen(
                            clientModel = clientModel,
                            onCancel = {
                                navController.popBackStack(Home, inclusive = false)
                            }
                        )
                    }
                }
            }
        }
    }
}
