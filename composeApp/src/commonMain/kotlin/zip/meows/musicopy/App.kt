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
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import zip.meows.musicopy.ui.screens.ConnectManuallyScreen
import zip.meows.musicopy.ui.screens.ConnectQRScreen
import zip.meows.musicopy.ui.screens.ConnectingScreen
import zip.meows.musicopy.ui.screens.HomeScreen

enum class AppScreen() {
    Home(),
    ConnectQR(),
    ConnectManually(),
    Connecting(),
    PreTransfer(),
    Transfer()
}

@Composable
fun App(
    viewModel: CoreViewModel = viewModel(),
    navController: NavHostController = rememberNavController(),
) {
    val model by viewModel.state.collectAsState()

    MaterialTheme {
        Scaffold() { innerPadding ->
            NavHost(
                navController = navController,
                startDestination = AppScreen.Home.name,
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
                composable(route = AppScreen.Home.name) {
                    // TODO: make this better...
                    model?.let {
                        HomeScreen(
                            model = it,
                            onConnectQRButtonClicked = { navController.navigate(AppScreen.ConnectQR.name) },
                            onConnectManuallyButtonClicked = { navController.navigate(AppScreen.ConnectManually.name) }
                        )
                    }
                }
                composable(route = AppScreen.ConnectQR.name) {
                    model?.let {
                        ConnectQRScreen(
                            onSubmit = { nodeId ->
                                viewModel.instance.connect(nodeId = nodeId)
                                navController.navigate(AppScreen.Connecting.name)
                            },
                            onCancel = {
                                navController.popBackStack(AppScreen.Home.name, inclusive = false)
                            }
                        )
                    }
                }
                composable(route = AppScreen.ConnectManually.name) {
                    model?.let {
                        ConnectManuallyScreen(
                            onSubmit = { nodeId ->
                                viewModel.instance.connect(nodeId = nodeId)
                                navController.navigate(AppScreen.Connecting.name)
                            },
                            onCancel = {
                                navController.popBackStack(AppScreen.Home.name, inclusive = false)
                            }
                        )
                    }
                }
                composable(route = AppScreen.Connecting.name) {
                    ConnectingScreen(
                        onCancel = {
                            navController.popBackStack(AppScreen.Home.name, inclusive = false)
                        }
                    )
                }
                composable(route = AppScreen.PreTransfer.name) {
                    Text("pretransfer")
                }
                composable(route = AppScreen.Transfer.name) {
                    Text("transfer")
                }
            }
        }
    }
}
