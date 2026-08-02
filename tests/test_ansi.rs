use crossterm::style::Stylize;
use rigging::render::ansi;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mid_word_cut_with_styles() {
        let full_text = format!(
            "Architecture Overview: The system relies on a {} for event delivery.",
            "HIGHLY_AVAILABLE_DISTRIBUTED_MESSAGE_BROKER"
                .red()
                .bold()
                .italic()
                .on_yellow()
        );

        let cut_point = full_text.find("DISTRIBUTED_").unwrap() + "DISTRIBUTED_".len();
        let first_line_slice = &full_text[..cut_point];

        let active_color = ansi::get_active_text_color(first_line_slice);

        assert!(
            active_color == Some("\x1b[31m".to_string())
                || active_color == Some("\x1b[38;5;9m".to_string()),
            "Unexpected color sequence: {:?}",
            active_color
        );
    }

    #[test]
    fn test_mid_word_cut_rgb() {
        let full_text = format!(
            "Fantasy Novel Chapter 1: The sky turned into a {}",
            "DEEP_PURPLE_SHADE_BEFORE_THE_STORM_STARTED".with(crossterm::style::Color::Rgb {
                r: 128,
                g: 0,
                b: 128,
            })
        );

        let cut_point = full_text.find("SHADE_").unwrap() + "SHADE_".len();
        let slice = &full_text[..cut_point];

        let active_color = ansi::get_active_text_color(slice);

        assert_eq!(active_color, Some("\x1b[38;2;128;0;128m".to_string()));
    }

    #[test]
    fn test_cut_after_full_reset() {
        let full_text = format!(
            "{} The subsequent execution pipeline proceeds sequentially without restrictions.",
            "CRITICAL_WARNING: Deprecated API used.".red()
        );

        let cut_point = full_text
            .find("execution_")
            .unwrap_or(full_text.find("pipeline").unwrap());
        let slice = &full_text[..cut_point];

        assert_eq!(ansi::get_active_text_color(slice), None);
    }

    #[test]
    fn test_real_line_wrap_flow() {
        let original = format!(
            "It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in {} across all domains.",
            "WANT_OF_A_VERY_SPECIFIC_AND_IMPORTANT_WIFE".blue().bold()
        );

        let cut_idx = original.find("IMPORTANT_").unwrap();
        let line_1 = &original[..cut_idx];

        let active_color = ansi::get_active_text_color(line_1).expect("Color should be active!");

        let line_2_raw = &original[cut_idx..];
        let line_2_formatted = format!("{active_color}{line_2_raw}");

        assert!(
            line_2_formatted.starts_with("\x1b[34mIMPORTANT_")
                || line_2_formatted.starts_with("\x1b[38;5;12mIMPORTANT_")
                || line_2_formatted.starts_with("\x1b[38;5;9mIMPORTANT_"),
            "Line 2 failed to preserve style prefix: {:?}",
            line_2_formatted
        );
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[test]
    fn test_stress_multiple_standalone_styles_before_cut() {
        // Цвет задан в начале, а прямо перед разрезом идут только стили (bold, italic, bg)
        // Функция должна пропустить "пустые" на цвет группы и докопаться до 31m!
        let text = "\x1b[31mRed Text\x1b[1m\x1b[3m\x1b[48;5;4mCut here";
        assert_eq!(
            ansi::get_active_text_color(text),
            Some("\x1b[31m".to_string())
        );
    }

    #[test]
    fn test_stress_reset_in_middle_of_complex_sequence() {
        // Внутри одной группы идет комбинация сброса и установки цвета
        // 1. Сначала цвет 31, потом сброс 0 -> None
        let text_reset_last = "\x1b[31;0mText";
        assert_eq!(ansi::get_active_text_color(text_reset_last), None);

        // 2. Сначала сброс 0, а за ним цвет 32 -> Green
        let text_color_last = "\x1b[0;32mText";
        assert_eq!(
            ansi::get_active_text_color(text_color_last),
            Some("\x1b[32m".to_string())
        );
    }

    #[test]
    fn test_stress_rgb_channels_with_zeros() {
        // RGB с нулевыми каналами R, G, B = 0 (черный/темный цвет).
        // Ни в коем случае не должен спутать нули каналов с ESC[0m!
        let pure_black_rgb = "\x1b[38;2;0;0;0mBlack text cut";
        assert_eq!(
            ansi::get_active_text_color(pure_black_rgb),
            Some("\x1b[38;2;0;0;0m".to_string())
        );

        let zero_green_rgb = "\x1b[38;2;255;0;128mText";
        assert_eq!(
            ansi::get_active_text_color(zero_green_rgb),
            Some("\x1b[38;2;255;0;128m".to_string())
        );
    }

    #[test]
    fn test_stress_256_color_palette() {
        // 8-битная палитра 38;5;N
        let text_256 = "\x1b[38;5;208mOrange text";
        assert_eq!(
            ansi::get_active_text_color(text_256),
            Some("\x1b[38;5;208m".to_string())
        );
    }

    #[test]
    fn test_stress_high_intensity_colors() {
        // Яркие цвета (Bright/High-Intensity) в диапазоне 90..=97
        let bright_cyan = "\x1b[96mBright Cyan Text";
        assert_eq!(
            ansi::get_active_text_color(bright_cyan),
            Some("\x1b[96m".to_string())
        );
    }

    #[test]
    fn test_stress_fg_reset_code_39() {
        // Код 39 — сброс ТОЛЬКО цвета текста (фоновый цвет 42m при этом остается)
        let text_with_fg_reset = "\x1b[31mRed\x1b[42mGreenBG\x1b[39mDefaultFG";
        assert_eq!(ansi::get_active_text_color(text_with_fg_reset), None);
    }

    #[test]
    fn test_stress_letter_m_in_plain_text() {
        // Проверка на фальшивые сработки: обычные буквы 'm' в тексте
        // "memory management" содержит кучу 'm' без ESC-последовательностей
        let tricky_text = "\x1b[33mWarning: system memory management algorithm";
        assert_eq!(
            ansi::get_active_text_color(tricky_text),
            Some("\x1b[33m".to_string())
        );
    }

    #[test]
    fn test_stress_empty_and_no_ansi() {
        // Вообще нет ANSI
        assert_eq!(
            ansi::get_active_text_color("Just plain text with no color"),
            None
        );
        assert_eq!(ansi::get_active_text_color(""), None);
    }
}
