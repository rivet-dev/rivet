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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingUser.js
var require_faBuildingUser = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "building-user";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4da";
    var svgPathData = "M64 64C64 28.7 92.7 0 128 0L384 0c35.3 0 64 28.7 64 64l0 121.3c-46.9 19-80 65-80 118.7 0 27.7 8.8 53.4 23.8 74.4-51.5 21-87.8 71.6-87.8 130.7 0 1 0 1.9 0 2.9l-176 0c-35.3 0-64-28.7-64-64L64 64zM208 400l0 64 53.4 0c7.2-29.4 21.3-56.1 40.4-78.3-6.1-19.5-24.3-33.7-45.8-33.7-26.5 0-48 21.5-48 48zM339 224.3c-1-.2-2-.3-3-.3l-32 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l16.7 0c2.1-22.8 8.4-44.3 18.3-63.7zM176 96c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0zm112 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zM176 224c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0zm240 80a80 80 0 1 1 160 0 80 80 0 1 1 -160 0zM352 512c0-53 43-96 96-96l96 0c53 0 96 43 96 96 0 17.7-14.3 32-32 32l-224 0c-17.7 0-32-14.3-32-32z";
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
    exports.faBuildingUser = exports.definition;
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

// src/_entries/faBuildingUser.js
var import_faBuildingUser = __toESM(require_faBuildingUser());
var export_faBuildingUser = import_faBuildingUser.definition;
export {
  export_faBuildingUser as faBuildingUser
};
