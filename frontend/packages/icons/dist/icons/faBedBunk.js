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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBedBunk.js
var require_faBedBunk = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBedBunk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bed-bunk";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8f8";
    var svgPathData = "M64 32C64 14.3 49.7 0 32 0S0 14.3 0 32L0 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 192 0 256 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 0-32 0-128 0-32 0-64 0-32 0-48C576 50.1 525.9 0 464 0L256 0c-17.7 0-32 14.3-32 32l0 96L64 128l0-96zM512 256l-256 0c-17.7 0-32 14.3-32 32l0 96L64 384l0-192 192 0 256 0 0 64zM192 304a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM144 96a48 48 0 1 0 0-96 48 48 0 1 0 0 96z";
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
    exports.faBedBunk = exports.definition;
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

// src/_entries/faBedBunk.js
var import_faBedBunk = __toESM(require_faBedBunk());
var export_faBedBunk = import_faBedBunk.definition;
export {
  export_faBedBunk as faBedBunk
};
