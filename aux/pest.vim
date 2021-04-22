" Vim syntax file
" Language: Billig DSL
" Maintainer: Neven Villani (Vanille-N)
" Lastest Revision: 21 April 2021

if exists("b:current_syntax")
    finish
endif

syn match pestWrapper '\(_\|\$\|@\|\){\|}'
syn match pestRange +'.'\.\.'.'+
syn match pestOperator '(\|)\|+\|*\|?\|!\||\|\~'
syn match pestRule '\([[:alpha:]]\|_\)\+'
syn match pestRepeat '{\([[:digit:]]\|,\)\+}'
syn match pestEscape contained '\\.'
syn region pestPattern start=/"/ skip=+\\\\\|\\"+ end=/"/ contains=pestEscape
syn keyword pestPredefs ANY COMMENT WHITESPACE SOI EOI
syn region pestComment start='//' end='\n'

let b:current_syntax = "pest"

hi def link pestEscape Special
hi def link pestPattern String
hi def link pestRange String
hi def link pestOperator Type
hi def link pestRule Operator
hi def link pestWrapper Statement
hi def link pestPredefs Todo
hi def link pestRepeat Type
hi def link pestComment Comment

