/**
 * @file Lambda grammar for tree-sitter
 * @author Brage Ingebrigtsen
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "lambda",

  inline: $ => [
    $.directive,
    $.group
  ],

  extras: $ => [
    /\s/,
    $.comment
  ],

  rules: {
    // TODO: add the actual grammar rules
    source_file: ($) =>
      seq(optional($.shebang), repeat($._statement)),

    shebang: () => seq("#!", /.*\r?\n?/),

    _statement: $ =>
      choice(
        $.directive,
        $.assignment,
        $._expression,
      ),

    _expression: $ =>
      choice(
        $.application,
        $._atom,
      ),

    _atom: $ =>
      choice(
        $.group,
        $.variable,
        $.lambda,
        $.ident,
      ),

    directive: $ =>
      seq("!", choice(
        $.source
      )),

    source: $ =>
      seq("source", $._expression, ";"),

    assignment: $ =>
      seq($.variable, "=", $._expression, ";"),

    application: $ =>
      prec.left(1, seq($._expression, $._atom)),

    variable: $ =>
      seq("$", $.ident),

    lambda: $ =>
      seq("L", field("param", $.ident), ".", $._expression),

    group: $ => prec(10, seq("(", $._expression, ")")),

    ident: $ =>
      token(prec(-1, /[^()\s=;!.#]+/)),

    comment: _ => token(seq("#", /[^\r\n]*/)),
  }
});
