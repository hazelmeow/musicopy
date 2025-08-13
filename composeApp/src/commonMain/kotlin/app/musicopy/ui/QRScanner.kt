package app.musicopy.ui

import androidx.compose.runtime.Composable

@Composable
expect fun QRScanner(onResult: (String) -> Unit);
