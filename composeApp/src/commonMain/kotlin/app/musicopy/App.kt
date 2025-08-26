package app.musicopy

import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import app.musicopy.ui.NodeStatusSheet
import app.musicopy.ui.Theme
import app.musicopy.ui.rememberNodeStatusSheetState
import app.musicopy.ui.screens.ConnectManually
import app.musicopy.ui.screens.ConnectManuallyScreen
import app.musicopy.ui.screens.ConnectQR
import app.musicopy.ui.screens.ConnectQRScreen
import app.musicopy.ui.screens.Disconnected
import app.musicopy.ui.screens.DisconnectedScreen
import app.musicopy.ui.screens.Home
import app.musicopy.ui.screens.HomeScreen
import app.musicopy.ui.screens.PreTransfer
import app.musicopy.ui.screens.PreTransferScreen
import app.musicopy.ui.screens.Transfer
import app.musicopy.ui.screens.TransferScreen
import app.musicopy.ui.screens.Waiting
import app.musicopy.ui.screens.WaitingScreen
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.musicopy.CoreException

@Composable
fun App(
    platformAppContext: PlatformAppContext,
    platformActivityContext: PlatformActivityContext,
    coreInstance: CoreInstance,
    navController: NavHostController = rememberNavController(),
) {
    val libraryModel by coreInstance.libraryState.collectAsState()
    val nodeModel by coreInstance.nodeState.collectAsState()

    val directoryPicker = remember { DirectoryPicker(platformActivityContext) }

    val scope = rememberCoroutineScope()
    var connectCount by remember { mutableStateOf(0) }
    val isConnecting = connectCount > 0

    val onConnect = { nodeId: String ->
        scope.launch {
            connectCount += 1
            try {
                coreInstance.instance.connect(nodeId = nodeId)
                delay(100) // TODO
                val client = nodeModel.clients.find { it.nodeId == nodeId }
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

    val leaveClientScreen = { nodeId: String ->
        coreInstance.instance.closeClient(nodeId)

        navController.popBackStack(Home, inclusive = false)
    }

    val nodeStatusSheetState = rememberNodeStatusSheetState()
    NodeStatusSheet(nodeStatusSheetState, nodeModel)
    val onShowNodeStatus = { nodeStatusSheetState.peek() }

    Theme {
        NavHost(
            navController = navController,
            startDestination = Home,
            // TODO
            // modifier = Modifier.verticalScroll(rememberScrollState()),
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
                HomeScreen(
                    onShowNodeStatus = onShowNodeStatus,

                    recentServers = nodeModel.recentServers,
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
                    },
                    onConnectRecent = onConnect,
                )
            }
            composable<ConnectQR> {
                ConnectQRScreen(
                    onShowNodeStatus = onShowNodeStatus,

                    isConnecting = isConnecting,
                    onSubmit = onConnect,
                    onCancel = {
                        navController.popBackStack(Home, inclusive = false)
                    },
                )

            }
            composable<ConnectManually> {
                ConnectManuallyScreen(
                    onShowNodeStatus = onShowNodeStatus,

                    isConnecting = isConnecting,
                    onSubmit = onConnect,
                    onCancel = {
                        navController.popBackStack(Home, inclusive = false)
                    }
                )

            }
            composable<Waiting> { backStackEntry ->
                val waiting: Waiting = backStackEntry.toRoute()
                val nodeId = waiting.nodeId
                val clientModel = nodeModel.clients.find { x -> x.nodeId == nodeId }

                if (clientModel?.accepted == true && navController.currentDestination?.hasRoute<Waiting>() == true) {
                    navController.navigate(PreTransfer(nodeId = nodeId)) {
                        // pop Waiting screen from back stack
                        popUpTo<Waiting>() {
                            inclusive = true
                        }
                    }
                }

                clientModel?.let { clientModel ->
                    WaitingScreen(
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        onCancel = {
                            leaveClientScreen(nodeId)
                        }
                    )

                }
            }
            composable<PreTransfer> { backStackEntry ->
                val preTransfer: PreTransfer = backStackEntry.toRoute()
                val nodeId = preTransfer.nodeId
                val clientModel = nodeModel.clients.find { x -> x.nodeId == nodeId }

                // TODO: clientmodel just have a disconnected flag instead of removing immediately
                if (clientModel == null) {
                    if (navController.currentDestination?.hasRoute<PreTransfer>() == true) {
                        // navigate to Disconnected screen
                        navController.navigate(Disconnected(nodeId = nodeId))
                    }
                } else {
                    val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(
                        null
                    )

                    PreTransferScreen(
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        onDownloadAll = {
                            downloadDirectory?.let { downloadDirectory ->
                                coreInstance.instance.downloadAll(nodeId, downloadDirectory)
                                navController.navigate(Transfer(nodeId = nodeId))
                            } ?: run {
                                // TODO toast?
                                println("download directory is null")
                            }
                        },
                        onDownloadPartial = { items ->
                            downloadDirectory?.let { downloadDirectory ->
                                coreInstance.instance.downloadPartial(
                                    nodeId,
                                    items,
                                    downloadDirectory
                                )
                                navController.navigate(Transfer(nodeId = nodeId))
                            } ?: run {
                                // TODO toast?
                                println("download directory is null")
                            }
                        },
                        onCancel = {
                            leaveClientScreen(nodeId)
                        }
                    )

                }
            }
            composable<Transfer> { backStackEntry ->
                val transfer: Transfer = backStackEntry.toRoute()
                val nodeId = transfer.nodeId
                val clientModel = nodeModel.clients.find { x -> x.nodeId == nodeId }

                if (clientModel == null) {
                    if (navController.currentDestination?.hasRoute<Transfer>() == true) {
                        // navigate to Disconnected screen
                        navController.navigate(Disconnected(nodeId = nodeId))
                    }
                } else {
                    TransferScreen(
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        onCancel = {
                            // pop back to pretransfer
                            navController.popBackStack(PreTransfer(nodeId), inclusive = false)
                        }
                    )
                }

            }
            composable<Disconnected> { backStackEntry ->
                val route: Disconnected = backStackEntry.toRoute()
                val nodeId = route.nodeId
                DisconnectedScreen(
                    onShowNodeStatus = onShowNodeStatus,

                    nodeId = nodeId,
                    isConnecting = isConnecting,
                    onReconnect = { onConnect(nodeId) },
                    onCancel = {
                        // pop back to home
                        navController.popBackStack(Home, inclusive = false)
                    }
                )
            }
        }
    }
}
