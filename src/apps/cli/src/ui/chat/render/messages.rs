impl ChatView {
fn render_messages(&mut self, frame: &mut Frame, area: Rect, chat_state: &ChatState) {
        let title = if self.browse_mode {
            format!(" Conversation [Browse Mode \u{2195}] ")
        } else {
            " Conversation ".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.style(StyleKind::Border))
            .style(Style::default().bg(self.theme.background))
            .title(title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Store messages area for mouse click hit-testing
        self.messages_area = Some(inner);
        // Regions are recalculated each frame for the currently rendered (visible subset) list.
        self.block_tool_regions.clear();
        self.thinking_regions.clear();
        let available_width = inner.width;

        if chat_state.messages.is_empty() {
            let welcome = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Welcome to northhing CLI!",
                    self.theme.style(StyleKind::Title),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter your request, AI will help you complete programming tasks.",
                    self.theme.style(StyleKind::Info),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Tip: Use / prefix for quick commands",
                    self.theme.style(StyleKind::Muted),
                )),
            ];

            let paragraph = Paragraph::new(welcome)
                .alignment(Alignment::Center)
                .style(Style::default().bg(self.theme.background))
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, inner);
            self.visible_plain_lines.clear();
            self.mouse.selection_anchor = None;
            self.mouse.selection_focus = None;
            self.mouse.selection_mouse_down = None;
            self.mouse.selection_dragged = false;
        } else {
            let visible_lines = inner.height as usize;

            // ── Step 1: Ensure all messages are in the render cache and collect line counts ──
            let mut msg_line_counts: Vec<usize> = Vec::with_capacity(chat_state.messages.len());
            for msg in &chat_state.messages {
                if msg.is_streaming {
                    // Streaming messages: always re-render
                    let rendered = self.render_message(msg, available_width);
                    let lc = rendered.items.len();
                    self.render_cache.insert(
                        msg.id.clone(),
                        MessageRenderEntry {
                            items: rendered.items,
                            line_count: lc,
                            version: msg.version,
                            width: available_width,
                            plain_lines: rendered.plain_lines,
                            tool_regions: rendered.tool_regions,
                            thinking_regions: rendered.thinking_regions,
                        },
                    );
                    msg_line_counts.push(lc);
                } else {
                    let cache_valid = self
                        .render_cache
                        .get(&msg.id)
                        .map(|e| e.version == msg.version && e.width == available_width)
                        .unwrap_or(false);

                    if cache_valid {
                        msg_line_counts.push(self.render_cache.get(&msg.id).unwrap().line_count);
                    } else {
                        let rendered = self.render_message(msg, available_width);
                        let lc = rendered.items.len();
                        self.render_cache.insert(
                            msg.id.clone(),
                            MessageRenderEntry {
                                items: rendered.items,
                                line_count: lc,
                                version: msg.version,
                                width: available_width,
                                plain_lines: rendered.plain_lines,
                                tool_regions: rendered.tool_regions,
                                thinking_regions: rendered.thinking_regions,
                            },
                        );
                        msg_line_counts.push(lc);
                    }
                }
            }

            // ── Step 2: Build prefix sum for line counts ──
            let total_lines: usize = msg_line_counts.iter().sum();

            // Update line count cache
            self.cached_total_lines = total_lines;
            self.cached_msg_count = chat_state.messages.len();
            self.cached_width = available_width;
            self.lines_cache_dirty = false;

            if total_lines == 0 {
                return;
            }

            // prefix_sum[i] = total lines of messages 0..i (exclusive end)
            // prefix_sum[0] = 0, prefix_sum[1] = msg_line_counts[0], etc.
            let mut prefix_sum: Vec<usize> = Vec::with_capacity(msg_line_counts.len() + 1);
            prefix_sum.push(0);
            for &lc in &msg_line_counts {
                prefix_sum.push(prefix_sum.last().unwrap() + lc);
            }

            // ── Step 3: Determine visible line range ──
            let view_start_line = if self.browse_mode {
                if self.scroll_offset >= total_lines {
                    0
                } else {
                    total_lines.saturating_sub(self.scroll_offset + visible_lines)
                }
            } else {
                // Auto-scroll: show bottom
                total_lines.saturating_sub(visible_lines)
            };

            let view_end_line =
                (view_start_line + visible_lines + visible_lines / 2).min(total_lines); // buffer: render half a screen extra

            // ── Step 4: Binary search for visible message range ──
            // Find first message that overlaps [view_start_line, view_end_line)
            let start_msg_idx = match prefix_sum.binary_search(&view_start_line) {
                Ok(i) => i.min(chat_state.messages.len().saturating_sub(1)),
                Err(i) => i.saturating_sub(1),
            };
            let end_msg_idx = match prefix_sum.binary_search(&view_end_line) {
                Ok(i) => i.min(chat_state.messages.len()),
                Err(i) => i.min(chat_state.messages.len()),
            };

            // ── Step 5: Collect ListItems only for visible messages ──
            // We need to include some items before view_start_line from the first
            // visible message (partial message visibility), so we collect from
            // start_msg_idx and let the List widget handle the offset.
            let lines_before_start_msg = prefix_sum[start_msg_idx];
            let offset_within_visible = view_start_line.saturating_sub(lines_before_start_msg);

            let mut messages: Vec<ListItem<'static>> = Vec::new();
            let mut visible_plain_lines: Vec<String> = Vec::new();
            let mut y_cursor: u16 = 0;
            for msg_idx in start_msg_idx..end_msg_idx {
                let msg = &chat_state.messages[msg_idx];
                if let Some(entry) = self.render_cache.get(&msg.id) {
                    messages.extend(entry.items.clone());
                    visible_plain_lines.extend(entry.plain_lines.clone());
                    for (tool_id, y_start, y_end) in &entry.tool_regions {
                        self.block_tool_regions.push((
                            tool_id.clone(),
                            y_cursor.saturating_add(*y_start),
                            y_cursor.saturating_add(*y_end),
                        ));
                    }
                    for (message_id, y_start, y_end) in &entry.thinking_regions {
                        self.thinking_regions.push((
                            message_id.clone(),
                            y_cursor.saturating_add(*y_start),
                            y_cursor.saturating_add(*y_end),
                        ));
                    }
                    y_cursor = y_cursor
                        .saturating_add(entry.items.len().min(u16::MAX as usize) as u16);
                }
            }

            // Apply hover styling (without invalidating per-message render caches)
            if let Some(ref hovered_id) = self.hovered_thinking_block_id {
                for (block_id, y_start, y_end) in &self.thinking_regions {
                    if block_id == hovered_id && y_start == y_end {
                        let idx = *y_start as usize;
                        if idx < messages.len() {
                            messages[idx] = messages[idx]
                                .clone()
                                .style(Style::default().bg(self.theme.block_bg_hover));
                        }
                    }
                }
            }

            self.visible_plain_lines = visible_plain_lines;

            // ── Step 6: Set scroll state ──
            // The List widget receives only the visible subset of items.
            // offset_within_visible tells it how many lines to skip from the top.
            *self.list_state.offset_mut() = offset_within_visible;

            if self.browse_mode {
                let selected_in_subset = offset_within_visible + visible_lines / 2;
                self.list_state.select(Some(
                    selected_in_subset.min(messages.len().saturating_sub(1)),
                ));
            } else if self.auto_scroll {
                self.list_state.select(Some(messages.len().saturating_sub(1)));
                self.scroll_offset = 0;
            }

            // ── Scroll indicator ──
            if self.browse_mode {
                let progress_pct = if self.scroll_offset == 0 {
                    100
                } else if self.scroll_offset >= total_lines {
                    0
                } else {
                    ((total_lines - self.scroll_offset) * 100 / total_lines).min(100)
                };

                let scroll_indicator = format!("{}%", progress_pct);
                let indicator_area = Rect {
                    x: inner.x + inner.width.saturating_sub(12),
                    y: inner.y,
                    width: 10,
                    height: 1,
                };

                let indicator_widget = Paragraph::new(scroll_indicator)
                    .style(self.theme.style(StyleKind::Info))
                    .alignment(Alignment::Right);
                frame.render_widget(indicator_widget, indicator_area);
            }

            let list = List::new(messages).highlight_style(Style::default());

            frame.render_stateful_widget(list, inner, &mut self.list_state);
            self.render_mouse_selection_overlay(frame, inner);
        }

        // Note: thinking indicator moved to status bar area (between Conversation and Input)
    }

    /// Render a single message into a list of owned ListItems.
    /// Returns owned items plus message-local clickable regions so results can be cached across frames.
    fn render_message(
        &mut self,
        message: &ChatMessage,
        available_width: u16,
    ) -> MessageRenderResult {
        let mut items: Vec<ListItem<'static>> = Vec::new();
        let mut plain_lines: Vec<String> = Vec::new();
        let mut tool_regions: Vec<(String, u16, u16)> = Vec::new();
        let mut thinking_regions: Vec<(String, u16, u16)> = Vec::new();
        let mut thinking_block_index: usize = 0;

        // Match opencode's TUI style: no explicit "You:" / "Assistant:" prefixes.
        // Instead, differentiate user messages via background color (and a subtle left border).
        let user_bg_style = Style::default().bg(self.theme.background_panel);
        let user_border_style = self
            .theme
            .style(StyleKind::Success)
            .add_modifier(Modifier::BOLD);

        fn blank_line() -> ListItem<'static> {
            ListItem::new(Line::from(Span::raw(String::new())))
        }

        fn user_padding_line(
            user_bg_style: Style,
            user_border_style: Style,
        ) -> ListItem<'static> {
            ListItem::new(Line::from(vec![
                Span::raw(" ".to_string()),
                Span::styled("\u{258f}".to_string(), user_border_style), // ▏
                Span::raw(" ".to_string()),
            ]))
            .style(user_bg_style)
        }

        fn close_user_bubble(
            items: &mut Vec<ListItem<'static>>,
            plain_lines: &mut Vec<String>,
            open: &mut bool,
            user_bg_style: Style,
            user_border_style: Style,
        ) {
            if *open {
                items.push(user_padding_line(user_bg_style, user_border_style));
                plain_lines.push(" | ".to_string());
                *open = false;
            }
        }

        fn wrap_hard_display_width(s: &str, max_width: usize) -> Vec<String> {
            if max_width == 0 {
                return vec![String::new()];
            }
            if UnicodeWidthStr::width(s) <= max_width {
                return vec![s.to_string()];
            }

            let mut lines: Vec<String> = Vec::new();
            let mut current = String::new();
            let mut current_width = 0usize;

            for ch in s.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

                if !current.is_empty() && current_width + ch_width > max_width {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }

                // Even if a single char is wider than max_width, still render it.
                current.push(ch);
                current_width += ch_width;

                if current_width >= max_width && !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
            }

            if !current.is_empty() {
                lines.push(current);
            }

            if lines.is_empty() {
                lines.push(String::new());
            }

            lines
        }

        // Top margin between messages (previously provided partly by role/timestamp line).
        items.push(blank_line());
        plain_lines.push(String::new());

        let spinner_frame = self.spinner.current().to_string();
        let mut user_bubble_open = false;

        if !message.flow_items.is_empty() {
            for flow_item in &message.flow_items {
                match flow_item {
                    FlowItem::Text {
                        content,
                        is_streaming,
                    } => {
                        if message.role == MessageRole::Assistant
                            && MarkdownRenderer::has_markdown_syntax(content)
                        {
                            close_user_bubble(
                                &mut items,
                                &mut plain_lines,
                                &mut user_bubble_open,
                                user_bg_style,
                                user_border_style,
                            );
                            let md_width = available_width.saturating_sub(2) as usize;
                            // Use cached render for completed messages, fresh render for streaming
                            let markdown_lines = if message.is_streaming {
                                self.markdown_renderer.render(content, md_width)
                            } else {
                                self.markdown_renderer.render_cached(content, md_width)
                            };

                            for md_line in markdown_lines {
                                let mut spans: Vec<Span<'static>> =
                                    vec![Span::raw("  ".to_string())];
                                spans.extend(md_line.spans);
                                let plain = spans
                                    .iter()
                                    .map(|span| span.content.as_ref())
                                    .collect::<String>();
                                items.push(ListItem::new(Line::from(spans)));
                                plain_lines.push(plain);
                            }
                        } else {
                            if message.role != MessageRole::User {
                                close_user_bubble(
                                    &mut items,
                                    &mut plain_lines,
                                    &mut user_bubble_open,
                                    user_bg_style,
                                    user_border_style,
                                );
                            }
                            for line in content.lines() {
                                if message.role == MessageRole::User {
                                    let max_text_width =
                                        available_width.saturating_sub(3) as usize;
                                    let wrapped = wrap_hard_display_width(line, max_text_width);
                                    if !user_bubble_open {
                                        items.push(user_padding_line(
                                            user_bg_style,
                                            user_border_style,
                                        ));
                                        plain_lines.push(" | ".to_string());
                                        user_bubble_open = true;
                                    }
                                    for wrapped_line in wrapped {
                                        let plain = format!(" | {}", wrapped_line);
                                        items.push(
                                            ListItem::new(Line::from(vec![
                                                Span::raw(" ".to_string()),
                                                Span::styled(
                                                    "\u{258f}".to_string(),
                                                    user_border_style,
                                                ), // ▏
                                                Span::raw(" ".to_string()),
                                                Span::raw(wrapped_line),
                                            ]))
                                            .style(user_bg_style),
                                        );
                                        plain_lines.push(plain);
                                    }
                                } else {
                                    let max_text_width =
                                        available_width.saturating_sub(2) as usize;
                                    for wrapped_line in
                                        wrap_hard_display_width(line, max_text_width)
                                    {
                                        let plain = format!("  {}", wrapped_line);
                                        items.push(ListItem::new(Line::from(vec![
                                            Span::raw("  ".to_string()),
                                            Span::raw(wrapped_line),
                                        ])));
                                        plain_lines.push(plain);
                                    }
                                }
                            }
                        }

                        if *is_streaming {
                            if message.role == MessageRole::User {
                                if !user_bubble_open {
                                    items.push(user_padding_line(
                                        user_bg_style,
                                        user_border_style,
                                    ));
                                    plain_lines.push(" | ".to_string());
                                    user_bubble_open = true;
                                }
                                items.push(
                                    ListItem::new(Line::from(vec![
                                        Span::raw(" ".to_string()),
                                        Span::styled(
                                            "\u{258f}".to_string(),
                                            user_border_style,
                                        ), // ▏
                                        Span::raw(" ".to_string()),
                                        Span::styled(
                                            "\u{2588}".to_string(),
                                            self.theme.style(StyleKind::Primary),
                                        ),
                                    ]))
                                    .style(user_bg_style),
                                );
                                plain_lines.push(" | _".to_string());
                            } else {
                                items.push(ListItem::new(Line::from(vec![
                                    Span::raw("  ".to_string()),
                                    Span::styled(
                                        "\u{2588}".to_string(),
                                        self.theme.style(StyleKind::Primary),
                                    ),
                                ])));
                                plain_lines.push("  _".to_string());
                            }
                        }
                    }

                    FlowItem::Thinking { content } => {
                        let thinking_block_id =
                            format!("{}::thinking:{}", message.id, thinking_block_index);
                        thinking_block_index = thinking_block_index.saturating_add(1);

                        close_user_bubble(
                            &mut items,
                            &mut plain_lines,
                            &mut user_bubble_open,
                            user_bg_style,
                            user_border_style,
                        );
                        // Render thinking block with distinct style.
                        // Use trailing <thinking_end> marker to auto-collapse once thinking is complete.
                        let trimmed = content.trim_end();
                        let has_end_marker = trimmed.ends_with("<thinking_end>");
                        let clean_content =
                            trimmed.trim_end_matches("<thinking_end>").trim_end();

                        let thinking_ended = has_end_marker || !message.is_streaming;
                        if thinking_ended
                            && !self
                                .selection
                                .thinking_user_overrides
                                .contains(&thinking_block_id)
                            && !self
                                .selection
                                .thinking_auto_collapsed
                                .contains(&thinking_block_id)
                        {
                            self.selection
                                .collapsed_thinking
                                .insert(thinking_block_id.clone());
                            self.selection
                                .thinking_auto_collapsed
                                .insert(thinking_block_id.clone());
                        }

                        let collapsed = self
                            .selection
                            .collapsed_thinking
                            .contains(&thinking_block_id);
                        let caret = if collapsed { "\u{25b8}" } else { "\u{25be}" }; // ▸ / ▾

                        let header_y = items.len().min(u16::MAX as usize) as u16;
                        thinking_regions.push((thinking_block_id.clone(), header_y, header_y));
                        let left_label = format!("{} Thinking", caret);
                        if collapsed {
                            let hint = "click to expand";
                            let indent = "  ";
                            let gap = (available_width as usize)
                                .saturating_sub(indent.width() + left_label.width() + hint.width());
                            let spacer = " ".repeat(gap.max(1));
                            let plain = format!("{}{}{}{}", indent, left_label, spacer, hint);
                            items.push(ListItem::new(Line::from(vec![
                                Span::raw(indent.to_string()),
                                Span::styled(
                                    left_label,
                                    self.theme
                                        .style(StyleKind::Muted)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                                Span::raw(spacer),
                                Span::styled(hint.to_string(), self.theme.style(StyleKind::Muted)),
                            ])));
                            plain_lines.push(plain);
                        } else {
                            let plain = format!("  {}", left_label);
                            items.push(ListItem::new(Line::from(vec![
                                Span::raw("  ".to_string()),
                                Span::styled(
                                    left_label,
                                    self.theme
                                        .style(StyleKind::Muted)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ])));
                            plain_lines.push(plain);
                        }

                        let content_lines: Vec<&str> = clean_content.lines().collect();
                        let line_count = content_lines.len();

                        if collapsed {
                            // Collapsed: header only (no extra summary lines)
                        } else if line_count == 0 {
                            items.push(ListItem::new(Line::from(vec![
                                Span::raw("    ".to_string()),
                                Span::styled(
                                    "(empty)".to_string(),
                                    self.theme.style(StyleKind::Muted),
                                ),
                            ])));
                            plain_lines.push("    (empty)".to_string());
                        } else {
                            let thinking_max_width =
                                available_width.saturating_sub(4) as usize; // 4 = indent "    "
                            for line in content_lines {
                                let wrapped =
                                    wrap_hard_display_width(line, thinking_max_width);
                                for wl in wrapped {
                                    let plain = format!("    {}", wl);
                                    items.push(ListItem::new(Line::from(vec![
                                        Span::raw("    ".to_string()),
                                        Span::styled(
                                            wl,
                                            self.theme.style(StyleKind::Muted),
                                        ),
                                    ])));
                                    plain_lines.push(plain);
                                }
                            }
                        }

                        // Extra spacing so thinking doesn't visually stick to following text/tools.
                        items.push(blank_line());
                        plain_lines.push(String::new());
                    }

                    FlowItem::Tool { tool_state } => {
                        close_user_bubble(
                            &mut items,
                            &mut plain_lines,
                            &mut user_bubble_open,
                            user_bg_style,
                            user_border_style,
                        );
                        let expanded =
                            !self.selection.collapsed_tools.contains(&tool_state.tool_id);
                        let focused = self.selection.focused_block_tool.as_ref()
                            == Some(&tool_state.tool_id);
                        let tool_render = crate::ui::tool_cards::render_tool_card(
                            tool_state,
                            &self.theme,
                            expanded,
                            focused,
                            &spinner_frame,
                            available_width,
                        );
                        let y_start = items.len().min(u16::MAX as usize) as u16;
                        items.extend(tool_render.items);
                        plain_lines.extend(tool_render.plain_lines);
                        let y_end = items
                            .len()
                            .saturating_sub(1)
                            .min(u16::MAX as usize) as u16;
                        tool_regions.push((tool_state.tool_id.clone(), y_start, y_end));
                    }
                }
            }
        } else {
            // Empty flow_items — shouldn't happen normally, but handle gracefully
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  ".to_string()),
                Span::styled("(empty)".to_string(), self.theme.style(StyleKind::Muted)),
            ])));
            plain_lines.push("  (empty)".to_string());
        }

        close_user_bubble(
            &mut items,
            &mut plain_lines,
            &mut user_bubble_open,
            user_bg_style,
            user_border_style,
        );

        // Bottom margin between messages (helps tool -> thinking transitions).
        items.push(blank_line());
        plain_lines.push(String::new());

        MessageRenderResult {
            items,
            tool_regions,
            thinking_regions,
            plain_lines,
        }
    }
}