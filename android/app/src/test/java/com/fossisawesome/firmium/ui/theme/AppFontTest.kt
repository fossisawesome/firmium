package com.fossisawesome.firmium.ui.theme

import org.junit.Assert.assertEquals
import org.junit.Test

class AppFontTest {

    @Test
    fun `bundled display names map to their bundled key`() {
        assertEquals(AppFontKey.INTER, fontKeyFor("Inter"))
        assertEquals(AppFontKey.LIBERATION_MONO, fontKeyFor("Liberation Mono"))
        assertEquals(AppFontKey.FIRACODE, fontKeyFor("FiraCode"))
        assertEquals(AppFontKey.HACK, fontKeyFor("Hack"))
        assertEquals(AppFontKey.COUSINE, fontKeyFor("Cousine"))
        assertEquals(AppFontKey.BIGBLUE_TERMINAL, fontKeyFor("BigBlue Terminal"))
    }

    @Test
    fun `generic names map to generic keys`() {
        assertEquals(AppFontKey.MONOSPACE, fontKeyFor("Monospace"))
        assertEquals(AppFontKey.SANS_SERIF, fontKeyFor("Sans Serif"))
        assertEquals(AppFontKey.COMIC_SANS, fontKeyFor("Comic Sans"))
    }

    @Test
    fun `system and iced map to default`() {
        assertEquals(AppFontKey.DEFAULT, fontKeyFor("System"))
        assertEquals(AppFontKey.DEFAULT, fontKeyFor("Iced"))
    }

    @Test
    fun `unknown name maps to default`() {
        assertEquals(AppFontKey.DEFAULT, fontKeyFor("Nonexistent Font"))
    }

    @Test
    fun `font options has eleven entries in dropdown order`() {
        assertEquals(11, FONT_OPTIONS.size)
        assertEquals("Inter", FONT_OPTIONS.first())
        assertEquals("Hack", FONT_OPTIONS.last())
    }
}
