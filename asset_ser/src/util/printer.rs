//! Pretty printer for loaded objects with cycle detection

use colored::*;
use std::collections::HashSet;
use std::fmt::Write;
use uesave::Property;

use crate::{
    core::object_pool::{AssetArchiveType, ObjectHandle, ObjectPool, ObjectRef},
    object::ObjectType,
};

/// Pretty printer for objects with cycle detection
pub struct ObjectPrinter<'a> {
    pool: &'a ObjectPool,
    visited: HashSet<ObjectHandle>,
    indent_level: usize,
    indent_str: String,
    use_colors: bool,
}

impl<'a> ObjectPrinter<'a> {
    /// Create a new object printer with colors enabled
    pub fn new(pool: &'a ObjectPool) -> Self {
        Self {
            pool,
            visited: HashSet::new(),
            indent_level: 0,
            indent_str: "  ".to_string(),
            use_colors: true,
        }
    }

    /// Set the indentation string (default is two spaces)
    pub fn with_indent(mut self, indent: impl Into<String>) -> Self {
        self.indent_str = indent.into();
        self
    }

    /// Enable or disable colored output
    pub fn with_colors(mut self, use_colors: bool) -> Self {
        self.use_colors = use_colors;
        self
    }

    /// Print an object by handle
    pub fn print_object(&mut self, handle: ObjectHandle) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        // Check if we've already visited this object (cycle detection)
        if self.visited.contains(&handle) {
            writeln!(output, "{}<cycle detected>", self.current_indent())?;
            return Ok(output);
        }

        // Get the object from the pool
        let Some(obj) = self.pool.get(handle) else {
            writeln!(output, "{}<invalid handle>", self.current_indent())?;
            return Ok(output);
        };

        // Mark as visited
        self.visited.insert(handle);

        // Print object header
        let obj_path = self.pool.build_path(handle);
        let class_path = self.pool.resolve_path(&obj.class);

        if self.use_colors {
            writeln!(
                output,
                "{}{}: {}",
                self.current_indent(),
                "Object".bright_cyan().bold(),
                obj_path.as_str().bright_white()
            )?;

            writeln!(
                output,
                "{}{}: {}",
                self.current_indent(),
                "Class".bright_cyan(),
                class_path.as_str().bright_yellow()
            )?;

            if let Some(template_ref) = &obj.template {
                writeln!(
                    output,
                    "{}{}: {}",
                    self.current_indent(),
                    "Template".bright_cyan(),
                    self.pool.resolve_path(template_ref).to_string().yellow()
                )?;
            }

            if let Some(outer_ref) = &obj.outer {
                writeln!(
                    output,
                    "{}{}: {}",
                    self.current_indent(),
                    "Outer".bright_cyan(),
                    self.pool.resolve_path(outer_ref).to_string().bright_black()
                )?;
            }

            writeln!(
                output,
                "{}{}:",
                self.current_indent(),
                "Properties".bright_cyan().bold()
            )?;
        } else {
            writeln!(output, "{}Object: {}", self.current_indent(), obj_path)?;

            writeln!(output, "{}Class: {}", self.current_indent(), class_path)?;

            if let Some(template_ref) = &obj.template {
                writeln!(
                    output,
                    "{}Template: {}",
                    self.current_indent(),
                    self.pool.resolve_path(template_ref)
                )?;
            }

            if let Some(outer_ref) = &obj.outer {
                writeln!(
                    output,
                    "{}Outer: {}",
                    self.current_indent(),
                    self.pool.resolve_path(outer_ref)
                )?;
            }

            writeln!(output, "{}Properties:", self.current_indent())?;
        }
        self.indent_level += 1;
        for (key, property) in obj.properties().0.iter() {
            write!(output, "{}", self.print_property(&key.1, property)?)?;
        }
        self.indent_level -= 1;

        // Print type-specific data
        write!(output, "{}", self.print_type_specific_data(&obj.object)?)?;

        // Unmark as visited (allow revisiting in different branches)
        self.visited.remove(&handle);

        Ok(output)
    }

    /// Print type-specific data for UStruct, UClass, UFunction
    fn print_type_specific_data(
        &mut self,
        object: &Box<dyn ObjectType>,
    ) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        // Try to downcast to specific types using the AsAny trait
        let any_ref = object.as_any();

        if let Some(uclass) = any_ref.downcast_ref::<crate::object::UClass>() {
            write!(output, "{}", self.print_uclass_data(uclass)?)?;
        } else if let Some(ufunc) = any_ref.downcast_ref::<crate::object::UFunction>() {
            write!(output, "{}", self.print_ufunction_data(ufunc)?)?;
        } else if let Some(ustruct) = any_ref.downcast_ref::<crate::object::UStruct>() {
            write!(output, "{}", self.print_ustruct_data(ustruct)?)?;
        }

        Ok(output)
    }

    /// Print UStruct-specific data
    fn print_ustruct_data(
        &mut self,
        ustruct: &crate::object::UStruct,
    ) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        if let Some(super_struct) = &ustruct.super_struct {
            self.print_header(&mut output, "SuperStruct")?;
            self.indent_level += 1;
            write!(output, "{}", self.print_object_ref(super_struct)?)?;
            self.indent_level -= 1;
        }

        if !ustruct.children.is_empty() {
            self.print_header_with_count(&mut output, "Children", ustruct.children.len())?;
            self.indent_level += 1;
            for (idx, child) in ustruct.children.iter().enumerate() {
                writeln!(output, "{}[{}]:", self.current_indent(), idx)?;
                self.indent_level += 1;
                write!(output, "{}", self.print_object_ref(child)?)?;
                self.indent_level -= 1;
            }
            self.indent_level -= 1;
        }

        if !ustruct.child_properties.is_empty() {
            self.print_header_with_count(
                &mut output,
                "ChildProperties",
                ustruct.child_properties.len(),
            )?;
            self.indent_level += 1;
            for prop in &ustruct.child_properties {
                writeln!(
                    output,
                    "{}{}: {:?}",
                    self.current_indent(),
                    prop.base.name,
                    prop.r#type
                )?;
            }
            self.indent_level -= 1;
        }

        if !ustruct.script.is_empty() {
            let s = format!("{} bytes", ustruct.script.len());
            self.print_kv(&mut output, "Script", &s, s.bright_cyan().bold())?;
        }

        Ok(output)
    }

    /// Print UFunction-specific data
    fn print_ufunction_data(
        &mut self,
        ufunc: &crate::object::UFunction,
    ) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        let s = format!("{:?}", ufunc.function_flags);
        self.print_kv(&mut output, "FunctionFlags", &s, s.bright_cyan().bold())?;

        // Print base UStruct data
        write!(output, "{}", self.print_ustruct_data(&ufunc.base)?)?;

        Ok(output)
    }

    /// Print UClass-specific data
    fn print_uclass_data(
        &mut self,
        uclass: &crate::object::UClass,
    ) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        let s = format!("{:?}", uclass.class_flags);
        self.print_kv(&mut output, "ClassFlags", &s, s.bright_cyan().bold())?;

        self.print_header(&mut output, "ClassWithin")?;
        self.indent_level += 1;
        write!(output, "{}", self.print_object_ref(&uclass.class_within)?)?;
        self.indent_level -= 1;

        if !uclass.class_config_name.is_empty() {
            self.print_kv(
                &mut output,
                "ClassConfigName",
                &uclass.class_config_name,
                uclass.class_config_name.bright_green(),
            )?;
        }

        self.print_header(&mut output, "ClassGeneratedBy")?;
        self.indent_level += 1;
        write!(
            output,
            "{}",
            self.print_object_ref(&uclass.class_generated_by)?
        )?;
        self.indent_level -= 1;

        if !uclass.func_map.is_empty() {
            self.print_header_with_count(&mut output, "FuncMap", uclass.func_map.len())?;
            self.indent_level += 1;
            for (name, obj_ref) in &uclass.func_map {
                writeln!(output, "{}{}:", self.current_indent(), name)?;
                self.indent_level += 1;
                write!(output, "{}", self.print_object_ref(obj_ref)?)?;
                self.indent_level -= 1;
            }
            self.indent_level -= 1;
        }

        if !uclass.interfaces.is_empty() {
            self.print_header_with_count(&mut output, "Interfaces", uclass.interfaces.len())?;
            self.indent_level += 1;
            for (idx, interface) in uclass.interfaces.iter().enumerate() {
                writeln!(
                    output,
                    "{}[{}] offset={} blueprint={}:",
                    self.current_indent(),
                    idx,
                    interface.pointer_offset,
                    interface.implemented_in_blueprint
                )?;
                self.indent_level += 1;
                write!(output, "{}", self.print_object_ref(&interface.class)?)?;
                self.indent_level -= 1;
            }
            self.indent_level -= 1;
        }

        self.print_header(&mut output, "ClassDefaultObject")?;
        self.indent_level += 1;
        write!(
            output,
            "{}",
            self.print_object_ref(&uclass.class_default_object)?
        )?;
        self.indent_level -= 1;

        // Print base UStruct data
        write!(output, "{}", self.print_ustruct_data(&uclass.base)?)?;

        Ok(output)
    }

    /// Print an ObjectRef
    pub fn print_object_ref(&mut self, obj_ref: &ObjectRef) -> Result<String, std::fmt::Error> {
        match obj_ref {
            ObjectRef::Loaded(handle) => {
                let mut output = String::new();
                writeln!(output, "{}[Loaded Object]", self.current_indent())?;
                self.indent_level += 1;
                write!(output, "{}", self.print_object(*handle)?)?;
                self.indent_level -= 1;
                Ok(output)
            }
            ObjectRef::Unloaded(path) => {
                let mut output = String::new();
                writeln!(output, "{}[Unloaded: {}]", self.current_indent(), path)?;
                Ok(output)
            }
        }
    }

    /// Print a property
    fn print_property(
        &mut self,
        name: &str,
        property: &Property<AssetArchiveType>,
    ) -> Result<String, std::fmt::Error> {
        use uesave::Property::*;

        let mut output = String::new();

        macro_rules! kv {
            ($v:expr, $color:ident) => {{
                let s = $v.to_string();
                self.print_kv(&mut output, name, &s, s.$color())?
            }};
            ($v:expr, $fmt:literal, $color:ident) => {{
                let s = format!($fmt, $v);
                self.print_kv(&mut output, name, &s, s.$color())?
            }};
        }

        match property {
            // Numeric types
            Int8(v) => kv!(v, bright_magenta),
            Int16(v) => kv!(v, bright_magenta),
            Int(v) => kv!(v, bright_magenta),
            Int64(v) => kv!(v, bright_magenta),
            UInt8(v) => kv!(v, bright_magenta),
            UInt16(v) => kv!(v, bright_magenta),
            UInt32(v) => kv!(v, bright_magenta),
            UInt64(v) => kv!(v, bright_magenta),
            Float(v) => kv!(v, bright_magenta),
            Double(v) => kv!(v, bright_magenta),
            Bool(v) => kv!(v, bright_blue),
            Byte(v) => kv!(v, "{:?}", yellow),
            Enum(v) => kv!(v, "{:?}", yellow),
            Name(v) => kv!(v, "\"{}\"", bright_green),
            Str(v) => kv!(v, "\"{}\"", bright_green),
            Text(v) => kv!(v, "{:?}", bright_green),

            // Struct - check if it's simple
            Struct(v) if self.is_simple_struct(v) => {
                let s = self.format_simple_struct(v)?;
                self.print_kv(&mut output, name, &s, s.cyan())?;
            }

            // Complex values - print on next line with indentation
            _ => {
                if self.use_colors {
                    writeln!(output, "{}{}: ", self.current_indent(), name.green().bold())?;
                } else {
                    writeln!(output, "{}{}: ", self.current_indent(), name)?;
                }
                self.indent_level += 1;
                self.print_property_value(property, &mut output)?;
                self.indent_level -= 1;
            }
        }

        Ok(output)
    }

    /// Print complex property values
    fn print_property_value(
        &mut self,
        property: &Property<AssetArchiveType>,
        output: &mut String,
    ) -> Result<(), std::fmt::Error> {
        use uesave::Property::*;

        match property {
            Object(v) => {
                write!(output, "{}", self.print_object_ref(v)?)?;
            }
            SoftObject(v) => {
                writeln!(output, "{}SoftObject: {:?}", self.current_indent(), v)?;
            }
            Struct(v) => {
                // Check if this is a simple struct (can be printed inline)
                if self.is_simple_struct(v) {
                    write!(output, "{}", self.format_simple_struct(v)?)?;
                } else {
                    write!(output, "{}", self.print_struct_properties(v)?)?;
                }
            }
            Array(v) => {
                write!(output, "{}", self.print_value_vec("Array", v)?)?;
            }
            Set(v) => {
                write!(output, "{}", self.print_value_vec("Set", v)?)?;
            }
            Map(entries) => {
                writeln!(
                    output,
                    "{}Map[{} items]:",
                    self.current_indent(),
                    entries.len()
                )?;
                self.indent_level += 1;
                for (idx, entry) in entries.iter().enumerate() {
                    writeln!(output, "{}Entry {}:", self.current_indent(), idx)?;
                    self.indent_level += 1;
                    write!(output, "{}", self.print_property("key", &entry.key)?)?;
                    write!(output, "{}", self.print_property("value", &entry.value)?)?;
                    self.indent_level -= 1;
                }
                self.indent_level -= 1;
            }
            Raw(bytes) => {
                writeln!(
                    output,
                    "{}Raw[{} bytes]",
                    self.current_indent(),
                    bytes.len()
                )?;
            }
            _ => {
                // Simple types are handled in print_property
            }
        }

        Ok(())
    }

    /// Print struct properties
    fn print_struct_properties(
        &mut self,
        struct_value: &uesave::StructValue<AssetArchiveType>,
    ) -> Result<String, std::fmt::Error> {
        use uesave::StructValue::*;

        let mut output = String::new();

        match struct_value {
            Struct(properties) => {
                for (key, property) in properties.0.iter() {
                    write!(output, "{}", self.print_property(&key.1, property)?)?;
                }
            }
            Guid(guid) => {
                writeln!(output, "{}Guid: {:?}", self.current_indent(), guid)?;
            }
            DateTime(dt) => {
                writeln!(output, "{}DateTime: {}", self.current_indent(), dt)?;
            }
            Timespan(ts) => {
                writeln!(output, "{}Timespan: {}", self.current_indent(), ts)?;
            }
            Vector2D(v) => {
                writeln!(
                    output,
                    "{}Vector2D: ({}, {})",
                    self.current_indent(),
                    v.x,
                    v.y
                )?;
            }
            Vector(v) => {
                writeln!(
                    output,
                    "{}Vector: ({}, {}, {})",
                    self.current_indent(),
                    v.x,
                    v.y,
                    v.z
                )?;
            }
            Box(b) => {
                writeln!(output, "{}Box:", self.current_indent())?;
                self.indent_level += 1;
                writeln!(
                    output,
                    "{}Min: ({}, {}, {})",
                    self.current_indent(),
                    b.min.x,
                    b.min.y,
                    b.min.z
                )?;
                writeln!(
                    output,
                    "{}Max: ({}, {}, {})",
                    self.current_indent(),
                    b.max.x,
                    b.max.y,
                    b.max.z
                )?;
                writeln!(output, "{}IsValid: {}", self.current_indent(), b.is_valid)?;
                self.indent_level -= 1;
            }
            IntPoint(p) => {
                writeln!(
                    output,
                    "{}IntPoint: ({}, {})",
                    self.current_indent(),
                    p.x,
                    p.y
                )?;
            }
            Quat(q) => {
                writeln!(
                    output,
                    "{}Quat: ({}, {}, {}, {})",
                    self.current_indent(),
                    q.x,
                    q.y,
                    q.z,
                    q.w
                )?;
            }
            Rotator(r) => {
                writeln!(
                    output,
                    "{}Rotator: (x={}, y={}, z={})",
                    self.current_indent(),
                    r.x,
                    r.y,
                    r.z
                )?;
            }
            LinearColor(c) => {
                writeln!(
                    output,
                    "{}LinearColor: (r={}, g={}, b={}, a={})",
                    self.current_indent(),
                    c.r,
                    c.g,
                    c.b,
                    c.a
                )?;
            }
            Color(c) => {
                writeln!(
                    output,
                    "{}Color: (r={}, g={}, b={}, a={})",
                    self.current_indent(),
                    c.r,
                    c.g,
                    c.b,
                    c.a
                )?;
            }
            other => {
                writeln!(output, "{}{:?}", self.current_indent(), other)?;
            }
        }

        Ok(output)
    }

    /// Print a ValueVec (Array or Set)
    fn print_value_vec(
        &mut self,
        collection_type: &str,
        value_vec: &uesave::ValueVec<AssetArchiveType>,
    ) -> Result<String, std::fmt::Error> {
        use uesave::ValueVec::*;

        let mut output = String::new();

        match value_vec {
            Object(obj_refs) => {
                writeln!(
                    output,
                    "{}{}[{} objects]:",
                    self.current_indent(),
                    collection_type,
                    obj_refs.len()
                )?;
                self.indent_level += 1;
                for (idx, obj_ref) in obj_refs.iter().enumerate() {
                    writeln!(output, "{}[{}]:", self.current_indent(), idx)?;
                    self.indent_level += 1;
                    write!(output, "{}", self.print_object_ref(obj_ref)?)?;
                    self.indent_level -= 1;
                }
                self.indent_level -= 1;
            }
            Struct(structs) => {
                writeln!(
                    output,
                    "{}{}[{} structs]:",
                    self.current_indent(),
                    collection_type,
                    structs.len()
                )?;
                self.indent_level += 1;
                for (idx, struct_value) in structs.iter().enumerate() {
                    writeln!(output, "{}[{}]:", self.current_indent(), idx)?;
                    self.indent_level += 1;
                    write!(output, "{}", self.print_struct_properties(struct_value)?)?;
                    self.indent_level -= 1;
                }
                self.indent_level -= 1;
            }
            // For other types, use debug formatting
            other => {
                writeln!(
                    output,
                    "{}{}: {:?}",
                    self.current_indent(),
                    collection_type,
                    other
                )?;
            }
        }

        Ok(output)
    }

    /// Check if a struct can be printed inline (simple value types)
    fn is_simple_struct(&self, struct_value: &uesave::StructValue<AssetArchiveType>) -> bool {
        use uesave::StructValue::*;

        matches!(
            struct_value,
            Guid(_)
                | DateTime(_)
                | Timespan(_)
                | Vector2D(_)
                | Vector(_)
                | IntPoint(_)
                | Quat(_)
                | Rotator(_)
                | LinearColor(_)
                | Color(_)
                | Box(_)
        )
    }

    /// Format a simple struct as an inline string
    fn format_simple_struct(
        &self,
        struct_value: &uesave::StructValue<AssetArchiveType>,
    ) -> Result<String, std::fmt::Error> {
        use uesave::StructValue::*;

        let result = match struct_value {
            Guid(guid) => format!("Guid({:?})", guid),
            DateTime(dt) => format!("DateTime({})", dt),
            Timespan(ts) => format!("Timespan({})", ts),
            Vector2D(v) => format!("Vector2D({}, {})", v.x, v.y),
            Vector(v) => format!("Vector({}, {}, {})", v.x, v.y, v.z),
            IntPoint(p) => format!("IntPoint({}, {})", p.x, p.y),
            Quat(q) => format!("Quat({}, {}, {}, {})", q.x, q.y, q.z, q.w),
            Rotator(r) => format!("Rotator({}, {}, {})", r.x, r.y, r.z),
            LinearColor(c) => format!("LinearColor({}, {}, {}, {})", c.r, c.g, c.b, c.a),
            Color(c) => format!("Color({}, {}, {}, {})", c.r, c.g, c.b, c.a),
            Box(b) => format!(
                "Box(min: ({}, {}, {}), max: ({}, {}, {}), valid: {})",
                b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z, b.is_valid
            ),
            _ => return Err(std::fmt::Error), // Not a simple struct
        };

        Ok(result)
    }

    /// Get the current indentation string
    fn current_indent(&self) -> String {
        self.indent_str.repeat(self.indent_level)
    }

    /// Print a simple key-value pair with appropriate coloring
    fn print_kv(
        &self,
        output: &mut String,
        name: &str,
        value: &str,
        colored: ColoredString,
    ) -> Result<(), std::fmt::Error> {
        if self.use_colors {
            writeln!(
                output,
                "{}{}: {}",
                self.current_indent(),
                name.green(),
                colored
            )
        } else {
            writeln!(output, "{}{}: {}", self.current_indent(), name, value)
        }
    }

    /// Print a section header (bold cyan label)
    fn print_header(&self, output: &mut String, label: &str) -> Result<(), std::fmt::Error> {
        if self.use_colors {
            writeln!(
                output,
                "{}{}:",
                self.current_indent(),
                label.bright_cyan().bold()
            )
        } else {
            writeln!(output, "{}{}:", self.current_indent(), label)
        }
    }

    /// Print a section header with a count
    fn print_header_with_count(
        &self,
        output: &mut String,
        label: &str,
        count: usize,
    ) -> Result<(), std::fmt::Error> {
        if self.use_colors {
            writeln!(
                output,
                "{}{} [{}]:",
                self.current_indent(),
                label.bright_cyan().bold(),
                count
            )
        } else {
            writeln!(output, "{}{} [{}]:", self.current_indent(), label, count)
        }
    }
}
