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
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import kotlinx.coroutines.launch
import uniffi.musicopy.CoreException
import zip.meows.musicopy.ui.screens.ConnectManually
import zip.meows.musicopy.ui.screens.ConnectManuallyScreen
import zip.meows.musicopy.ui.screens.ConnectQR
import zip.meows.musicopy.ui.screens.ConnectQRScreen
import zip.meows.musicopy.ui.screens.Home
import zip.meows.musicopy.ui.screens.WaitingScreen
import zip.meows.musicopy.ui.screens.HomeScreen
import zip.meows.musicopy.ui.screens.PreTransfer
import zip.meows.musicopy.ui.screens.PreTransferScreen
import zip.meows.musicopy.ui.screens.Transfer
import zip.meows.musicopy.ui.screens.Waiting

@Composable
fun App(
    platformContext: PlatformContext,
    viewModel: CoreViewModel = viewModel(),
    navController: NavHostController = rememberNavController(),
) {
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
                navController.navigate(Waiting(nodeId = nodeId))
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
        Scaffold() { innerPadding ->
            NavHost(
                navController = navController,
                startDestination = Home,
                modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())
                    .padding(innerPadding),
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
                                directoryPicker.pickDownloadDirectory()
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
                        val pendingClient =
                            model.node.pendingClients.find { x -> x.nodeId == nodeId }
                        val activeClient = model.node.activeClients.find { x -> x.nodeId == nodeId }

                        if (activeClient != null) {
                            navController.navigate(PreTransfer(nodeId = nodeId))
                        }

                        pendingClient?.let { clientModel ->
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
                        val activeClient = model.node.activeClients.find { x -> x.nodeId == nodeId }

                        activeClient?.let { clientModel ->
                            PreTransferScreen(
                                clientModel = clientModel,
                                onCancel = {
                                    navController.popBackStack(Home, inclusive = false)
                                }
                            )
                        }
                    }
                }
                composable<Transfer> {
                    Text("transfer")
                }
            }
        }
    }
}
