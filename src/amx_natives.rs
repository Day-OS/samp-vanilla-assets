use samp::prelude::*;
use samp::raw::types::AMX;

#[repr(i32)]
pub enum TextDrawFont {
    Font0 = 0,
    Font1 = 1,
    Font2 = 2,
    Font3 = 3,
    SpriteDrawFont = 4,
    ModelPreviewFont = 5,
}

fn call_native(amx: &Amx, name: &str, args: &[i32]) -> AmxResult<i32> {
    let index = amx.find_native(name).map_err(|e| {
        log::error!("Failed to find native {}: {:?}", name, e);
        e
    })?;
    let amx_ptr: *mut AMX = amx.amx().as_ptr();

    let mut params = Vec::with_capacity(args.len() + 1);
    params.push((args.len() * std::mem::size_of::<i32>()) as i32);
    params.extend_from_slice(args);

    // log::info!("Calling native {} with params: {:?}", name, params);
    let mut result: i32 = 0;
    unsafe {
        let callback = (*amx_ptr).callback;
        let err = callback(amx_ptr, index, &mut result, params.as_mut_ptr());
    }

    Ok(result)
}

pub fn create_object(
    amx: &Amx,
    modelid: i32,
    x: f32,
    y: f32,
    z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    draw_distance: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateObject",
        &[
            modelid,
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
            draw_distance.as_cell(),
        ],
    )
}

pub fn destroy_object(amx: &Amx, object_id: i32) -> AmxResult<i32> {
    destroy_dynamic_object(amx, object_id)
}

pub fn set_object_pos(amx: &Amx, object_id: i32, x: f32, y: f32, z: f32) -> AmxResult<i32> {
    set_dynamic_object_pos(amx, object_id, x, y, z)
}

pub fn attach_object_to_object(
    amx: &Amx,
    object_id: i32,
    parent_id: i32,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    sync_rotation: i32,
) -> AmxResult<i32> {
    attach_dynamic_object_to_object(
        amx,
        object_id,
        parent_id,
        offset_x,
        offset_y,
        offset_z,
        rotation_x,
        rotation_y,
        rotation_z,
        sync_rotation,
    )
}

pub fn add_simple_model(
    amx: &Amx,
    virtual_world: i32,
    base_id: i32,
    new_id: i32,
    dff: &str,
    texture_library: &str,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let dff = allocator.allot_string(dff)?;
    let texture_library = allocator.allot_string(texture_library)?;

    call_native(
        amx,
        "AddSimpleModel",
        &[
            virtual_world,
            base_id,
            new_id,
            dff.as_cell(),
            texture_library.as_cell(),
        ],
    )
}

pub fn play_audio_stream_for_player(
    amx: &Amx,
    player_id: &i32,
    url: &str,
    pos: Option<(f32, f32, f32)>,
    distance: f32,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let url = allocator.allot_string(url)?;

    let pos_x = pos.map_or(0.0, |p| p.0);
    let pos_y = pos.map_or(0.0, |p| p.1);
    let pos_z = pos.map_or(0.0, |p| p.2);
    let use_pos = if pos.is_some() { 1 } else { 0 };

    call_native(
        amx,
        "PlayAudioStreamForPlayer",
        &[
            *player_id,
            url.as_cell(),
            pos_x.as_cell(),
            pos_y.as_cell(),
            pos_z.as_cell(),
            distance.as_cell(),
            use_pos,
        ],
    )
}

pub fn stop_audio_stream_for_player(amx: &Amx, player_id: i32) -> AmxResult<i32> {
    call_native(amx, "StopAudioStreamForPlayer", &[player_id])
}

pub fn show_player_dialog(
    amx: &Amx,
    player_id: i32,
    dialog_id: i32,
    style: i32,
    title: &str,
    body: &str,
    button1: &str,
    button2: &str,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let title = allocator.allot_string(title)?;
    let body = allocator.allot_string(body)?;
    let button1 = allocator.allot_string(button1)?;
    let button2 = allocator.allot_string(button2)?;

    call_native(
        amx,
        "ShowPlayerDialog",
        &[
            player_id,
            dialog_id,
            style,
            title.as_cell(),
            body.as_cell(),
            button1.as_cell(),
            button2.as_cell(),
        ],
    )
}

pub fn hide_player_dialog(amx: &Amx, player_id: i32) -> AmxResult<i32> {
    call_native(amx, "HidePlayerDialog", &[player_id])
}

pub fn text_draw_create(amx: &Amx, x: f32, y: f32, text: &str) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let text = allocator.allot_string(text)?;

    call_native(
        amx,
        "TextDrawCreate",
        &[x.as_cell(), y.as_cell(), text.as_cell()],
    )
}

pub fn text_draw_font(amx: &Amx, textdraw_id: i32, font: i32) -> AmxResult<i32> {
    call_native(amx, "TextDrawFont", &[textdraw_id, font])
}

pub fn text_draw_text_size(amx: &Amx, textdraw_id: i32, width: f32, height: f32) -> AmxResult<i32> {
    call_native(
        amx,
        "TextDrawTextSize",
        &[textdraw_id, width.as_cell(), height.as_cell()],
    )
}

pub fn text_draw_show_for_all(amx: &Amx, textdraw_id: i32) -> AmxResult<i32> {
    call_native(amx, "TextDrawShowForAll", &[textdraw_id])
}

pub fn text_draw_show_for_player(amx: &Amx, player_id: i32, textdraw_id: i32) -> AmxResult<i32> {
    call_native(amx, "TextDrawShowForPlayer", &[player_id, textdraw_id])
}

pub fn set_object_material_text(
    amx: &Amx,
    object_id: i32,
    text: &str,
    material_index: i32,
    material_size: i32,
    font_face: &str,
    font_size: i32,
    bold: i32,
    font_colour: i32,
    background_colour: i32,
    text_alignment: i32,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let text = allocator.allot_string(text)?;
    let font_face = allocator.allot_string(font_face)?;

    call_native(
        amx,
        "SetObjectMaterialText",
        &[
            object_id,
            text.as_cell(),
            material_index,
            material_size,
            font_face.as_cell(),
            font_size,
            bold,
            font_colour,
            background_colour,
            text_alignment,
        ],
    )
}

pub fn get_players(amx: &Amx, max_players: usize) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let array = allocator.allot_buffer(max_players)?;

    let result = call_native(amx, "GetPlayers", &[array.as_cell(), (max_players as i32)])?;

    let mut players = Vec::new();
    for i in 0..result as usize {
        if i < array.len() {
            players.push(array[i]);
        }
    }

    Ok(players)
}

pub fn get_player_pos(amx: &Amx, player_id: i32) -> AmxResult<(f32, f32, f32)> {
    let allocator = amx.allocator();
    let x = allocator.allot_buffer(1)?;
    let y = allocator.allot_buffer(1)?;
    let z = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "GetPlayerPos",
        &[player_id, x.as_cell(), y.as_cell(), z.as_cell()],
    )?;

    let x_val = f32::from_bits(x[0] as u32);
    let y_val = f32::from_bits(y[0] as u32);
    let z_val = f32::from_bits(z[0] as u32);

    Ok((x_val, y_val, z_val))
}

pub fn create_dynamic_object(
    amx: &Amx,
    modelid: i32,
    x: f32,
    y: f32,
    z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
    streamdistance: f32,
    drawdistance: f32,
    areaid: i32,
    priority: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicObject",
        &[
            modelid,
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
            worldid,
            interiorid,
            playerid,
            streamdistance.as_cell(),
            drawdistance.as_cell(),
            areaid,
            priority,
        ],
    )
}

pub fn destroy_dynamic_object(amx: &Amx, objectid: i32) -> AmxResult<i32> {
    call_native(amx, "DestroyDynamicObject", &[objectid])
}

pub fn is_valid_dynamic_object(amx: &Amx, objectid: i32) -> AmxResult<i32> {
    call_native(amx, "IsValidDynamicObject", &[objectid])
}

pub fn get_dynamic_object_pos(amx: &Amx, objectid: i32) -> AmxResult<(f32, f32, f32)> {
    let allocator = amx.allocator();
    let x = allocator.allot_buffer(1)?;
    let y = allocator.allot_buffer(1)?;
    let z = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "GetDynamicObjectPos",
        &[objectid, x.as_cell(), y.as_cell(), z.as_cell()],
    )?;

    let x_val = f32::from_bits(x[0] as u32);
    let y_val = f32::from_bits(y[0] as u32);
    let z_val = f32::from_bits(z[0] as u32);

    Ok((x_val, y_val, z_val))
}

pub fn set_dynamic_object_pos(amx: &Amx, objectid: i32, x: f32, y: f32, z: f32) -> AmxResult<i32> {
    call_native(
        amx,
        "SetDynamicObjectPos",
        &[objectid, x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn get_dynamic_object_rot(amx: &Amx, objectid: i32) -> AmxResult<(f32, f32, f32)> {
    let allocator = amx.allocator();
    let rx = allocator.allot_buffer(1)?;
    let ry = allocator.allot_buffer(1)?;
    let rz = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "GetDynamicObjectRot",
        &[objectid, rx.as_cell(), ry.as_cell(), rz.as_cell()],
    )?;

    let rx_val = f32::from_bits(rx[0] as u32);
    let ry_val = f32::from_bits(ry[0] as u32);
    let rz_val = f32::from_bits(rz[0] as u32);

    Ok((rx_val, ry_val, rz_val))
}

pub fn set_dynamic_object_rot(
    amx: &Amx,
    objectid: i32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "SetDynamicObjectRot",
        &[
            objectid,
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
        ],
    )
}

pub fn move_dynamic_object(
    amx: &Amx,
    objectid: i32,
    x: f32,
    y: f32,
    z: f32,
    speed: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "MoveDynamicObject",
        &[
            objectid,
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            speed.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
        ],
    )
}

pub fn stop_dynamic_object(amx: &Amx, objectid: i32) -> AmxResult<i32> {
    call_native(amx, "StopDynamicObject", &[objectid])
}

pub fn is_dynamic_object_moving(amx: &Amx, objectid: i32) -> AmxResult<i32> {
    call_native(amx, "IsDynamicObjectMoving", &[objectid])
}

pub fn attach_camera_to_dynamic_object(amx: &Amx, playerid: i32, objectid: i32) -> AmxResult<i32> {
    call_native(amx, "AttachCameraToDynamicObject", &[playerid, objectid])
}

pub fn attach_dynamic_object_to_object(
    amx: &Amx,
    objectid: i32,
    attachtoid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    syncrotation: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicObjectToObject",
        &[
            objectid,
            attachtoid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
            syncrotation,
        ],
    )
}

pub fn attach_dynamic_object_to_player(
    amx: &Amx,
    objectid: i32,
    playerid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicObjectToPlayer",
        &[
            objectid,
            playerid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
        ],
    )
}

pub fn attach_dynamic_object_to_vehicle(
    amx: &Amx,
    objectid: i32,
    vehicleid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicObjectToVehicle",
        &[
            objectid,
            vehicleid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
            rotation_x.as_cell(),
            rotation_y.as_cell(),
            rotation_z.as_cell(),
        ],
    )
}

pub fn is_dynamic_object_material_used(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "IsDynamicObjectMaterialUsed",
        &[objectid, materialindex],
    )
}

pub fn remove_dynamic_object_material(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "RemoveDynamicObjectMaterial",
        &[objectid, materialindex],
    )
}

pub fn get_dynamic_object_material(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<(i32, String, String, i32)> {
    let allocator = amx.allocator();
    let modelid = allocator.allot_buffer(1)?;
    let txdname = allocator.allot_buffer(16)?;
    let texturename = allocator.allot_buffer(16)?;
    let materialcolor = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "GetDynamicObjectMaterial",
        &[
            objectid,
            materialindex,
            modelid.as_cell(),
            txdname.as_cell(),
            16,
            texturename.as_cell(),
            16,
            materialcolor.as_cell(),
        ],
    )?;

    let model = modelid[0];
    let colour = materialcolor[0];

    let lib_str = AmxString::from_raw(amx, txdname.as_cell())?;
    let tex_str = AmxString::from_raw(amx, texturename.as_cell())?;

    Ok((model, lib_str.to_string(), tex_str.to_string(), colour))
}

pub fn set_dynamic_object_material(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
    modelid: i32,
    txdname: &str,
    texturename: &str,
    materialcolor: i32,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let txdname = allocator.allot_string(txdname)?;
    let texturename = allocator.allot_string(texturename)?;

    call_native(
        amx,
        "SetDynamicObjectMaterial",
        &[
            objectid,
            materialindex,
            modelid,
            txdname.as_cell(),
            texturename.as_cell(),
            materialcolor,
        ],
    )
}

pub fn is_dynamic_object_material_text_used(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "IsDynamicObjectMaterialTextUsed",
        &[objectid, materialindex],
    )
}

pub fn remove_dynamic_object_material_text(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "RemoveDynamicObjectMaterialText",
        &[objectid, materialindex],
    )
}

pub fn get_dynamic_object_material_text(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
) -> AmxResult<(String, i32, String, i32, bool, i32, i32, i32)> {
    let allocator = amx.allocator();
    let text = allocator.allot_buffer(128)?;
    let materialsize = allocator.allot_buffer(1)?;
    let fontface = allocator.allot_buffer(16)?;
    let fontsize = allocator.allot_buffer(1)?;
    let bold = allocator.allot_buffer(1)?;
    let fontcolor = allocator.allot_buffer(1)?;
    let backcolor = allocator.allot_buffer(1)?;
    let textalignment = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "GetDynamicObjectMaterialText",
        &[
            objectid,
            materialindex,
            text.as_cell(),
            materialsize.as_cell(),
            fontface.as_cell(),
            fontsize.as_cell(),
            bold.as_cell(),
            fontcolor.as_cell(),
            backcolor.as_cell(),
            textalignment.as_cell(),
            128,
            16,
        ],
    )?;

    let text_str = AmxString::from_raw(amx, text.as_cell())?;
    let font_str = AmxString::from_raw(amx, fontface.as_cell())?;

    Ok((
        text_str.to_string(),
        materialsize[0],
        font_str.to_string(),
        fontsize[0],
        bold[0] != 0,
        fontcolor[0],
        backcolor[0],
        textalignment[0],
    ))
}

pub fn set_dynamic_object_material_text(
    amx: &Amx,
    objectid: i32,
    materialindex: i32,
    text: &str,
    materialsize: i32,
    fontface: &str,
    fontsize: i32,
    bold: i32,
    fontcolor: i32,
    backcolor: i32,
    textalignment: i32,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let text = allocator.allot_string(text)?;
    let fontface = allocator.allot_string(fontface)?;

    call_native(
        amx,
        "SetDynamicObjectMaterialText",
        &[
            objectid,
            materialindex,
            text.as_cell(),
            materialsize,
            fontface.as_cell(),
            fontsize,
            bold,
            fontcolor,
            backcolor,
            textalignment,
        ],
    )
}

pub fn get_player_camera_target_dyn_object(amx: &Amx, playerid: i32) -> AmxResult<i32> {
    call_native(amx, "GetPlayerCameraTargetDynObject", &[playerid])
}

pub fn streamer_get_distance_to_item(
    amx: &Amx,
    x: f32,
    y: f32,
    z: f32,
    item_type: i32,
    id: i32,
    dimensions: i32,
) -> AmxResult<f32> {
    let allocator = amx.allocator();
    let distance = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "Streamer_GetDistanceToItem",
        &[
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            item_type,
            id,
            distance.as_cell(),
            dimensions,
        ],
    )?;

    Ok(f32::from_bits(distance[0] as u32))
}

pub fn streamer_toggle_item(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    id: i32,
    toggle: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_ToggleItem",
        &[playerid, item_type, id, toggle],
    )
}

pub fn streamer_is_toggle_item(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    id: i32,
) -> AmxResult<i32> {
    call_native(amx, "Streamer_IsToggleItem", &[playerid, item_type, id])
}

pub fn streamer_toggle_all_items(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    toggle: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_ToggleAllItems",
        &[playerid, item_type, toggle],
    )
}

pub fn streamer_get_item_internal_id(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    streamerid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_GetItemInternalID",
        &[playerid, item_type, streamerid],
    )
}

pub fn streamer_get_item_streamer_id(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    internalid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_GetItemStreamerID",
        &[playerid, item_type, internalid],
    )
}

pub fn streamer_is_item_visible(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    id: i32,
) -> AmxResult<i32> {
    call_native(amx, "Streamer_IsItemVisible", &[playerid, item_type, id])
}

pub fn streamer_destroy_all_visible_items(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    serverwide: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_DestroyAllVisibleItems",
        &[playerid, item_type, serverwide],
    )
}

pub fn streamer_count_visible_items(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    serverwide: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_CountVisibleItems",
        &[playerid, item_type, serverwide],
    )
}

pub fn streamer_destroy_all_items(amx: &Amx, item_type: i32, serverwide: i32) -> AmxResult<i32> {
    call_native(amx, "Streamer_DestroyAllItems", &[item_type, serverwide])
}

pub fn streamer_count_items(amx: &Amx, item_type: i32, serverwide: i32) -> AmxResult<i32> {
    call_native(amx, "Streamer_CountItems", &[item_type, serverwide])
}

pub fn streamer_get_nearby_items(
    amx: &Amx,
    x: f32,
    y: f32,
    z: f32,
    item_type: i32,
    range: f32,
    worldid: i32,
    max_items: usize,
) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let items = allocator.allot_buffer(max_items)?;

    let result = call_native(
        amx,
        "Streamer_GetNearbyItems",
        &[
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            item_type,
            items.as_cell(),
            (max_items as i32),
            range.as_cell(),
            worldid,
        ],
    )?;

    let mut found_items = Vec::new();
    for i in 0..result as usize {
        if i < items.len() {
            found_items.push(items[i]);
        }
    }

    Ok(found_items)
}

pub fn streamer_get_all_visible_items(
    amx: &Amx,
    playerid: i32,
    item_type: i32,
    max_items: usize,
) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let items = allocator.allot_buffer(max_items)?;

    let result = call_native(
        amx,
        "Streamer_GetAllVisibleItems",
        &[playerid, item_type, items.as_cell(), (max_items as i32)],
    )?;

    let mut visible_items = Vec::new();
    for i in 0..result as usize {
        if i < items.len() {
            visible_items.push(items[i]);
        }
    }

    Ok(visible_items)
}

pub fn streamer_get_item_pos(amx: &Amx, item_type: i32, id: i32) -> AmxResult<(f32, f32, f32)> {
    let allocator = amx.allocator();
    let x = allocator.allot_buffer(1)?;
    let y = allocator.allot_buffer(1)?;
    let z = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "Streamer_GetItemPos",
        &[item_type, id, x.as_cell(), y.as_cell(), z.as_cell()],
    )?;

    let x_val = f32::from_bits(x[0] as u32);
    let y_val = f32::from_bits(y[0] as u32);
    let z_val = f32::from_bits(z[0] as u32);

    Ok((x_val, y_val, z_val))
}

pub fn streamer_set_item_pos(
    amx: &Amx,
    item_type: i32,
    id: i32,
    x: f32,
    y: f32,
    z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_SetItemPos",
        &[item_type, id, x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn streamer_get_item_offset(amx: &Amx, item_type: i32, id: i32) -> AmxResult<(f32, f32, f32)> {
    let allocator = amx.allocator();
    let x = allocator.allot_buffer(1)?;
    let y = allocator.allot_buffer(1)?;
    let z = allocator.allot_buffer(1)?;

    call_native(
        amx,
        "Streamer_GetItemOffset",
        &[item_type, id, x.as_cell(), y.as_cell(), z.as_cell()],
    )?;

    let x_val = f32::from_bits(x[0] as u32);
    let y_val = f32::from_bits(y[0] as u32);
    let z_val = f32::from_bits(z[0] as u32);

    Ok((x_val, y_val, z_val))
}

pub fn streamer_set_item_offset(
    amx: &Amx,
    item_type: i32,
    id: i32,
    x: f32,
    y: f32,
    z: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "Streamer_SetItemOffset",
        &[item_type, id, x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn create_dynamic_circle(
    amx: &Amx,
    x: f32,
    y: f32,
    size: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
    priority: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicCircle",
        &[
            x.as_cell(),
            y.as_cell(),
            size.as_cell(),
            worldid,
            interiorid,
            playerid,
            priority,
        ],
    )
}

pub fn create_dynamic_cylinder(
    amx: &Amx,
    x: f32,
    y: f32,
    minz: f32,
    maxz: f32,
    size: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicCylinder",
        &[
            x.as_cell(),
            y.as_cell(),
            minz.as_cell(),
            maxz.as_cell(),
            size.as_cell(),
            worldid,
            interiorid,
            playerid,
        ],
    )
}

pub fn create_dynamic_sphere(
    amx: &Amx,
    x: f32,
    y: f32,
    z: f32,
    size: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicSphere",
        &[
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            size.as_cell(),
            worldid,
            interiorid,
            playerid,
        ],
    )
}

pub fn create_dynamic_rectangle(
    amx: &Amx,
    minx: f32,
    miny: f32,
    maxx: f32,
    maxy: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicRectangle",
        &[
            minx.as_cell(),
            miny.as_cell(),
            maxx.as_cell(),
            maxy.as_cell(),
            worldid,
            interiorid,
            playerid,
        ],
    )
}

pub fn create_dynamic_cuboid(
    amx: &Amx,
    minx: f32,
    miny: f32,
    minz: f32,
    maxx: f32,
    maxy: f32,
    maxz: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "CreateDynamicCuboid",
        &[
            minx.as_cell(),
            miny.as_cell(),
            minz.as_cell(),
            maxx.as_cell(),
            maxy.as_cell(),
            maxz.as_cell(),
            worldid,
            interiorid,
            playerid,
        ],
    )
}

pub fn create_dynamic_polygon(
    amx: &Amx,
    points: &[f32],
    minz: f32,
    maxz: f32,
    worldid: i32,
    interiorid: i32,
    playerid: i32,
) -> AmxResult<i32> {
    let allocator = amx.allocator();
    let points_buf = allocator.allot_buffer(points.len())?;

    unsafe {
        let ptr = points_buf.as_ptr() as *mut i32;
        for (i, &point) in points.iter().enumerate() {
            *ptr.add(i) = point.as_cell();
        }
    }

    call_native(
        amx,
        "CreateDynamicPolygon",
        &[
            points_buf.as_cell(),
            minz.as_cell(),
            maxz.as_cell(),
            (points.len() as i32),
            worldid,
            interiorid,
            playerid,
        ],
    )
}

pub fn destroy_dynamic_area(amx: &Amx, areaid: i32) -> AmxResult<i32> {
    call_native(amx, "DestroyDynamicArea", &[areaid])
}

pub fn is_valid_dynamic_area(amx: &Amx, areaid: i32) -> AmxResult<i32> {
    call_native(amx, "IsValidDynamicArea", &[areaid])
}

pub fn get_dynamic_area_type(amx: &Amx, areaid: i32) -> AmxResult<i32> {
    call_native(amx, "GetDynamicAreaType", &[areaid])
}

pub fn get_dynamic_polygon_points(
    amx: &Amx,
    areaid: i32,
    max_points: usize,
) -> AmxResult<Vec<f32>> {
    let allocator = amx.allocator();
    let points = allocator.allot_buffer(max_points)?;

    let result = call_native(
        amx,
        "GetDynamicPolygonPoints",
        &[areaid, points.as_cell(), (max_points as i32)],
    )?;

    let mut polygon_points = Vec::new();
    for i in 0..result as usize {
        if i < points.len() {
            polygon_points.push(f32::from_bits(points[i] as u32));
        }
    }

    Ok(polygon_points)
}

pub fn get_dynamic_polygon_number_points(amx: &Amx, areaid: i32) -> AmxResult<i32> {
    call_native(amx, "GetDynamicPolygonNumberPoints", &[areaid])
}

pub fn is_player_in_dynamic_area(
    amx: &Amx,
    playerid: i32,
    areaid: i32,
    recheck: i32,
) -> AmxResult<i32> {
    call_native(amx, "IsPlayerInDynamicArea", &[playerid, areaid, recheck])
}

pub fn is_player_in_any_dynamic_area(amx: &Amx, playerid: i32, recheck: i32) -> AmxResult<i32> {
    call_native(amx, "IsPlayerInAnyDynamicArea", &[playerid, recheck])
}

pub fn is_any_player_in_dynamic_area(amx: &Amx, areaid: i32, recheck: i32) -> AmxResult<i32> {
    call_native(amx, "IsAnyPlayerInDynamicArea", &[areaid, recheck])
}

pub fn is_any_player_in_any_dynamic_area(amx: &Amx, recheck: i32) -> AmxResult<i32> {
    call_native(amx, "IsAnyPlayerInAnyDynamicArea", &[recheck])
}

pub fn get_player_dynamic_areas(amx: &Amx, playerid: i32, max_areas: usize) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let areas = allocator.allot_buffer(max_areas)?;

    let result = call_native(
        amx,
        "GetPlayerDynamicAreas",
        &[playerid, areas.as_cell(), (max_areas as i32)],
    )?;

    let mut player_areas = Vec::new();
    for i in 0..result as usize {
        if i < areas.len() {
            player_areas.push(areas[i]);
        }
    }

    Ok(player_areas)
}

pub fn get_player_number_dynamic_areas(amx: &Amx, playerid: i32) -> AmxResult<i32> {
    call_native(amx, "GetPlayerNumberDynamicAreas", &[playerid])
}

pub fn is_point_in_dynamic_area(amx: &Amx, areaid: i32, x: f32, y: f32, z: f32) -> AmxResult<i32> {
    call_native(
        amx,
        "IsPointInDynamicArea",
        &[areaid, x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn is_point_in_any_dynamic_area(amx: &Amx, x: f32, y: f32, z: f32) -> AmxResult<i32> {
    call_native(
        amx,
        "IsPointInAnyDynamicArea",
        &[x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn is_line_in_dynamic_area(
    amx: &Amx,
    areaid: i32,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "IsLineInDynamicArea",
        &[
            areaid,
            x1.as_cell(),
            y1.as_cell(),
            z1.as_cell(),
            x2.as_cell(),
            y2.as_cell(),
            z2.as_cell(),
        ],
    )
}

pub fn is_line_in_any_dynamic_area(
    amx: &Amx,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "IsLineInAnyDynamicArea",
        &[
            x1.as_cell(),
            y1.as_cell(),
            z1.as_cell(),
            x2.as_cell(),
            y2.as_cell(),
            z2.as_cell(),
        ],
    )
}

pub fn get_dynamic_areas_for_point(
    amx: &Amx,
    x: f32,
    y: f32,
    z: f32,
    max_areas: usize,
) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let areas = allocator.allot_buffer(max_areas)?;

    let result = call_native(
        amx,
        "GetDynamicAreasForPoint",
        &[
            x.as_cell(),
            y.as_cell(),
            z.as_cell(),
            areas.as_cell(),
            (max_areas as i32),
        ],
    )?;

    let mut point_areas = Vec::new();
    for i in 0..result as usize {
        if i < areas.len() {
            point_areas.push(areas[i]);
        }
    }

    Ok(point_areas)
}

pub fn get_number_dynamic_areas_for_point(amx: &Amx, x: f32, y: f32, z: f32) -> AmxResult<i32> {
    call_native(
        amx,
        "GetNumberDynamicAreasForPoint",
        &[x.as_cell(), y.as_cell(), z.as_cell()],
    )
}

pub fn get_dynamic_areas_for_line(
    amx: &Amx,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
    max_areas: usize,
) -> AmxResult<Vec<i32>> {
    let allocator = amx.allocator();
    let areas = allocator.allot_buffer(max_areas)?;

    let result = call_native(
        amx,
        "GetDynamicAreasForLine",
        &[
            x1.as_cell(),
            y1.as_cell(),
            z1.as_cell(),
            x2.as_cell(),
            y2.as_cell(),
            z2.as_cell(),
            areas.as_cell(),
            (max_areas as i32),
        ],
    )?;

    let mut line_areas = Vec::new();
    for i in 0..result as usize {
        if i < areas.len() {
            line_areas.push(areas[i]);
        }
    }

    Ok(line_areas)
}

pub fn get_number_dynamic_areas_for_line(
    amx: &Amx,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "GetNumberDynamicAreasForLine",
        &[
            x1.as_cell(),
            y1.as_cell(),
            z1.as_cell(),
            x2.as_cell(),
            y2.as_cell(),
            z2.as_cell(),
        ],
    )
}

pub fn attach_dynamic_area_to_object(
    amx: &Amx,
    areaid: i32,
    objectid: i32,
    area_type: i32,
    playerid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicAreaToObject",
        &[
            areaid,
            objectid,
            area_type,
            playerid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
        ],
    )
}

pub fn attach_dynamic_area_to_player(
    amx: &Amx,
    areaid: i32,
    playerid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicAreaToPlayer",
        &[
            areaid,
            playerid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
        ],
    )
}

pub fn attach_dynamic_area_to_vehicle(
    amx: &Amx,
    areaid: i32,
    vehicleid: i32,
    offsetx: f32,
    offsety: f32,
    offsetz: f32,
) -> AmxResult<i32> {
    call_native(
        amx,
        "AttachDynamicAreaToVehicle",
        &[
            areaid,
            vehicleid,
            offsetx.as_cell(),
            offsety.as_cell(),
            offsetz.as_cell(),
        ],
    )
}

pub fn toggle_dyn_area_spectate_mode(amx: &Amx, areaid: i32, toggle: i32) -> AmxResult<i32> {
    call_native(amx, "ToggleDynAreaSpectateMode", &[areaid, toggle])
}

pub fn is_toggle_dyn_area_spectate_mode(amx: &Amx, areaid: i32) -> AmxResult<i32> {
    call_native(amx, "IsToggleDynAreaSpectateMode", &[areaid])
}
