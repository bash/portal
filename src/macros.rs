#[macro_export]
#[doc(hidden)]
macro_rules! state_enum {
    ($vis:vis $name:ident {$($body:tt)*} state $state:ident ($($field_ident:ident : $field_ty:ty),*) { execute $params:tt -> $return_ty:ty { $exec_block:expr } next ($ui_param:ident) $next_block:tt } $($tail:tt)*) => {
        $crate::state_enum! {
            $vis
            $name
            {
                $($body)*
                $state(Promise<$return_ty>, $($field_ty),*),
            }
            $($tail)*
        }
    };
    ($vis:vis $name:ident {$($body:tt)*} state $state:ident ($($field_ident:ident : $field_ty:ty),*) { } $($tail:tt)*) => {
        $crate::state_enum! {
            $vis
            $name
            {
                $($body)*
                $state($($field_ty),*),
            }
            $($tail)*
        }
    };
    ($vis:vis $name:ident {$($body:tt)*}) => {
        $vis enum $name { $($body)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! state_next_impl {
    ($ui:ident $target:ident {$($body:tt)*} state $state:ident ($($field_ident:ident : $field_ty:ty),*) { execute $params:tt -> $return_ty:ty { $exec_block:expr } next ($ui_param:ident) $match_block:tt } $($tail:tt)*) => {
        $crate::state_next_impl! {
            $ui
            $target
            {
                $($body)*
                $state(promise, $($field_ident),*) => match promise.try_take() {
                    Ok(result) => { let $ui_param = $ui; match result $match_block },
                    Err(promise) => $state(promise, $($field_ident),*),
                },
            }
            $($tail)*
        }
    };
    ($ui:ident $target:ident {$($body:tt)*} state $state:ident ($($field_ident:ident : $field_ty:ty),*) { } $($tail:tt)*) => {
        $crate::state_next_impl! {
            $ui
            $target
            { $($body)* }
            $($tail)*
        }
    };
    ($ui:ident $target:ident {$($body:tt)*}) => {
        match $target {
            $($body)*
            _ => $target,
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! state_new_fn {
    ($name:ident $state:ident ($($field_ident:ident : $field_ty:ty),*) { execute ($($param_name:ident : $param_ty:ty),*) -> $return_ty:ty { $exec_block:expr } next ($ui_param:ident) $next_block:tt }) => {
        ::paste::paste! {
            fn [<new_$state:snake>] (ui: &mut Ui, $($param_name : $param_ty,)* $($field_ident : $field_ty),*) -> $name {
                let promise = ui.ctx().spawn_async::<$return_ty>(async move { $exec_block });
                $name::$state(promise, $($field_ident),*)
            }
        }
    };
    ($name:ident $state:ident ($($field_ident:ident : $field_ty:ty),*) { }) => {
    };
}

#[macro_export]
macro_rules! states {
    (enum $name:ident; $(state $state:ident $params:tt $block:tt)*) => {
        crate::states!(pub(self) enum $name; $(state $state $params $block)*);
    };
    ($vis:vis enum $name:ident; $(state $state:ident $params:tt $block:tt)*) => {
        $crate::state_enum!($vis $name {} $(state $state $params $block)*);
        impl $name {
            fn next(&mut self, ui: &mut Ui) {
                use $name::*;
                ::take_mut::take(self, |state| {
                    $crate::state_next_impl!(ui state { } $(state $state $params $block)*)
                });
            }

            $(
                $crate::state_new_fn!($name $state $params $block);
            )*
        }
    };
}
