package zip.meows.musicopy

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform