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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckPlow.js
var require_faTruckPlow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckPlow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-plow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7de";
    var svgPathData = "M192 96l0 96 135.5 0L269.9 96 192 96zm-64 96l0-112c0-26.5 21.5-48 48-48l93.9 0c22.5 0 43.3 11.8 54.9 31.1L402.1 192l45.9 0c17.7 0 32 14.3 32 32l0 64 32 0 0-112c0-7.3 2.5-14.3 7-20l64-80c11-13.8 31.2-16 45-5s16 31.2 5 45l-57 71.2L576 320l0 36.8L633 428c11 13.8 8.8 33.9-5 45s-33.9 8.8-45-5l-64-80c-4.5-5.7-7-12.7-7-20l0-16-37.5 0c3.5 10 5.5 20.8 5.5 32c0 53-43 96-96 96s-96-43-96-96c0-11.2 1.9-22 5.5-32L224 352c-1.9 0-3.8-.2-5.6-.5c3.7 10.2 5.6 21.1 5.6 32.5c0 53-43 96-96 96s-96-43-96-96c0-11.2 1.9-22 5.5-32L32 352c-17.7 0-32-14.3-32-32l0-96c0-17.7 14.3-32 32-32l96 0zm32 192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm256 0a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faTruckPlow = exports.definition;
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

// src/_entries/faTruckPlow.js
var import_faTruckPlow = __toESM(require_faTruckPlow());
var export_faTruckPlow = import_faTruckPlow.definition;
export {
  export_faTruckPlow as faTruckPlow
};
