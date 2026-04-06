var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlbumCircleUser.js
var require_faAlbumCircleUser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlbumCircleUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "album-circle-user";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e48d";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 96.7c-5.3-.5-10.6-.7-16-.7c-20.1 0-39.5 3.4-57.5 9.6C352.3 140 293.3 96 224 96C135.6 96 64 167.6 64 256s71.6 160 160 160c12.9 0 25.4-1.5 37.4-4.4c6.5 25.5 18.5 48.7 34.8 68.4L64 480c-35.3 0-64-28.7-64-64L0 96zM224 224a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm64 144a144 144 0 1 1 288 0 144 144 0 1 1 -288 0zm221.7 80.7c-6.2-19-24-32.7-45.1-32.7l-65.2 0c-21 0-38.9 13.7-45.1 32.7C374.5 468.1 401.8 480 432 480s57.5-11.9 77.7-31.3zM480 336a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faAlbumCircleUser = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faAlbumCircleUser.js
var import_faAlbumCircleUser = __toESM(require_faAlbumCircleUser());
var export_faAlbumCircleUser = import_faAlbumCircleUser.definition;
export {
  export_faAlbumCircleUser as faAlbumCircleUser
};
