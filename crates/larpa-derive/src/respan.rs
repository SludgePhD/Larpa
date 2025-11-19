use proc_macro2::{
    Group as Group2, Ident as Ident2, Literal as Literal2, Punct as Punct2, Span,
    TokenStream as TokenStream2, TokenTree as TokenTree2,
};

pub trait Respan {
    /// Changes the [`Span`] of every token and delimiter in `self`, recursively.
    fn respan(self, new_span: Span) -> Self;
}

impl Respan for Ident2 {
    fn respan(mut self, new_span: Span) -> Self {
        self.set_span(new_span);
        self
    }
}

impl Respan for Punct2 {
    fn respan(mut self, new_span: Span) -> Self {
        self.set_span(new_span);
        self
    }
}

impl Respan for Literal2 {
    fn respan(mut self, new_span: Span) -> Self {
        self.set_span(new_span);
        self
    }
}

impl Respan for Group2 {
    fn respan(self, new_span: Span) -> Self {
        let mut g = Group2::new(self.delimiter(), self.stream().respan(new_span));
        g.set_span(new_span);
        g
    }
}

impl Respan for TokenTree2 {
    fn respan(self, new_span: Span) -> Self {
        match self {
            TokenTree2::Group(group) => group.respan(new_span).into(),
            TokenTree2::Ident(ident) => ident.respan(new_span).into(),
            TokenTree2::Punct(punct) => punct.respan(new_span).into(),
            TokenTree2::Literal(literal) => literal.respan(new_span).into(),
        }
    }
}

impl Respan for TokenStream2 {
    fn respan(self, new_span: Span) -> Self {
        self.into_iter().map(|tt| tt.respan(new_span)).collect()
    }
}
