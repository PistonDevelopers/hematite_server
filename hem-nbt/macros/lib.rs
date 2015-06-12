//! This crate contains a compiler plugin to generate NBT binary format
//! serialization code for custom structs. It can be used as follows:
//!
//! ```ignore
//! #![feature(plugin, custom_derive)]
//! #![plugin(nbt_macros)]
//! 
//! extern crate nbt;
//! 
//! use nbt::NbtError;
//! use nbt::serialize::{NbtFmt, to_writer};
//! 
//! #[derive(NbtFmt)]
//! struct MyMob {
//! 	name: String,
//! 	health: i8
//! }
//!
//! fn main() {
//!     let mut bytes = Vec::new();
//!     let mob = MyMob { name: "Dr. Evil".to_string(), health: 240 };
//!
//!     to_writer(&mut bytes, mob).unwrap();
//! }
//! ```
//! 
//! The custom `derive(NbtFmt)` will generate code equivalent to the following:
//! 
//! ```ignore
//! impl NbtFmt for MyMob {
//! 	fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
//! 	   where W: std::io::Write
//! 	{
//! 		try!(self.name.write_nbt_fmt_with_name(dst, "name"));
//!         try!(self.health.write_nbt_fmt_with_name(dst, "health"));
//! 
//!         close_nbt(dst)
//! 	}
//! }
//! ```
//! 
//! Which will work so long as the fields of the struct have `NbtFmt`
//! implementations of their own.

#![feature(plugin_registrar, quote, rustc_private)]

extern crate rustc;
extern crate syntax;

use syntax::ast::{Expr, MetaItem, Mutability};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt, MultiDecorator};
use syntax::ext::build::AstBuilder;
use syntax::ext::deriving::generic::*;
use syntax::ext::deriving::generic::ty::*;
use syntax::parse::token::{get_ident, InternedString};
use syntax::ptr::P;


#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(reg: &mut rustc::plugin::Registry) {
    reg.register_syntax_extension(
        syntax::parse::token::intern("derive_NbtFmt"),
        MultiDecorator(Box::new(expand_derive_nbtfmt)));
}

macro_rules! pathvec {
    ($($x:ident)::+) => (
        vec![ $( stringify!($x) ),+ ]
    )
}

macro_rules! path {
    ($($x:tt)*) => (
        Path::new( pathvec!( $($x)* ) )
    )
}

macro_rules! pathexpr {
	($cx:ident, $span:ident, $($x:ident)::+) => (
		$cx.expr_path($cx.path_global($span,
			vec![ $( $cx.ident_of( stringify!($x) ) ),+ ]))
	)
}

pub fn expand_derive_nbtfmt(cx: &mut ExtCtxt, span: Span, meta_item: &MetaItem,
                            item: &Annotatable,
                            push: &mut FnMut(Annotatable))
{
	let w_arg = Path::new_local("__W");
    let trait_def = TraitDef {
        span: span,
        attributes: Vec::new(),
        path: path!(nbt::serialize::NbtFmt),
        additional_bounds: Vec::new(),
        generics: LifetimeBounds::empty(),
        methods: vec!(
            MethodDef {
                name: "write_nbt_fmt",
                generics: LifetimeBounds {
                    lifetimes: Vec::new(),
                    // This adds a <__W: std::io::Writer> generic to the method.
                    bounds: vec![("__W", vec![path!(std::io::Write)])],
                },
                // Pass the immutable borrowed self argument, `&self`.
                explicit_self: borrowed_explicit_self(),
                // Pass a single argument of type `&mut __W`.
                args: vec!(Ptr(Box::new(Literal(w_arg)),
                               Borrowed(None, Mutability::MutMutable))),
                // Return a `Result<(), nbt::NbtError>`.
                ret_ty: Literal(Path::new_(
                    pathvec!(std::result::Result),
                    None,
                    vec!(Box::new(Tuple(Vec::new())), // Unit.
                         Box::new(Literal(Path::new_( // nbt::NbtError
                             pathvec!(nbt::NbtError),
                             None, Vec::new(), true)))),
                    true)),
                attributes: Vec::new(),
                is_unsafe: false,
                combine_substructure: combine_substructure(Box::new(|c, s, sub| {
                    cs_nbtfmt(c, s, sub)
                })),
            }
        ),
        associated_types: Vec::new(),
    };

    trait_def.expand(cx, meta_item, item, push)
}

fn cs_nbtfmt(cx: &mut ExtCtxt, trait_span: Span, substr: &Substructure) -> P<Expr> {
    // Retrieve the argument passed to the write_nbt_fmt function, i.e. the
    // `dst: &mut __W` bit. Since the method is already defined, there's no
    // reason for this to fail, so we call `cx.span_bug` indicating a compiler
    // error.
    let dst_expr = match (substr.nonself_args.len(),
                          substr.nonself_args.get(0)) {
        (1, Some(dst)) => dst,
        _ => cx.span_bug(trait_span,
                         "incorrect number of arguments in `derive(NbtFmt)`")
    };
    
    let call_nbt_fmt = |span, thing_expr, name| {
        let nbt_fmt_path = pathexpr!(cx, span, nbt::serialize::NbtFmt::write_nbt_fmt_with_name);
        let ref_thing = cx.expr_addr_of(span, thing_expr);
        
        // Create a string literal expression for the field's identifier.
        let name_lit = get_ident(name);
        let name_expr = cx.expr_str(span, name_lit);
        
        // Create a call expression, using the function path (nbt_fmt_path)
        // and `&self, dst, "<field>"` as arguments.
        let fmt_call = cx.expr_call(span, nbt_fmt_path,
                                    vec!(ref_thing, dst_expr.clone(), name_expr));

        // Wrap the call in a try! macro.
        let try_fmt_call = cx.expr_try(span, fmt_call);

        // Add a semicolon.
        cx.stmt_expr(try_fmt_call)
    };
    
    match *substr.fields {
        Struct(ref fields) => {   
        	// Unit structs are kind of irrelevant for NBT, so throw an error
        	// if someone tries to derive(NbtFmt) over one.         
            if fields.is_empty() {
                cx.span_err(trait_span,
                            "`NbtFmt` has no meaning for unit structs.");
                cx.expr_fail(trait_span, InternedString::new(""))
            } else if fields[0].name.is_none() {
            	// FIXME: Handle tuple structs using write_nbt_fmt_with_name
            	// with name = "".
                cx.span_err(trait_span,
                            "`NbtFmt` cannot yet be derived for tuple structs.");
                cx.expr_fail(trait_span, InternedString::new(""))
            } else {
                let mut stmts = Vec::new();

                for &FieldInfo { ref self_, span, name, .. } in fields {
                	// FIXME: Use cx.bug for properly handling unnamed fields.
                    stmts.push(call_nbt_fmt(span, self_.clone(), name.unwrap().clone()));
                }

                let close_nbt_path = pathexpr!(cx, trait_span, nbt::serialize::close_nbt);

                let close = cx.expr_call(trait_span, close_nbt_path, vec![dst_expr.clone()]);
                
                cx.expr_block(cx.block(trait_span, stmts, Some(close)))
            }
        },
        EnumMatching(..) => {
            cx.span_err(trait_span,
                        "`NbtFmt` cannot yet be derived for enums.");
            cx.expr_fail(trait_span, InternedString::new(""))
        },
        EnumNonMatchingCollapsed(..) => {
            cx.span_bug(trait_span,
                        "non-matching enum variants in `#[derive(NbtFmt)]`")
        },
        StaticEnum(..) | StaticStruct(..) => {
            cx.span_bug(trait_span, "static method in `#[derive(NbtFmt)]`")
        },
    }
}
