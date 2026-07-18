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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faOrnament.js
var require_faOrnament = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faOrnament.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ornament";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f7b8";
    var svgPathData = "M192 32a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm55.4 64c5.4-9.4 8.6-20.3 8.6-32c0-35.3-28.7-64-64-64s-64 28.7-64 64c0 11.7 3.1 22.6 8.6 32L128 96c-17.7 0-32 14.3-32 32l0 25.7C66.9 170.5 42.5 194.9 25.7 224l332.6 0c-16.9-29.1-41.2-53.5-70.3-70.3l0-25.7c0-17.7-14.3-32-32-32l-8.6 0zM0 320c0 22.4 3.8 44 10.9 64l362.2 0c7.1-20 10.9-41.6 10.9-64s-3.8-44-10.9-64L10.9 256C3.8 276 0 297.6 0 320zm25.7 96c33.2 57.4 95.2 96 166.3 96s133.1-38.6 166.3-96L25.7 416z";
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
    exports.faOrnament = exports.definition;
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

// src/_entries/faOrnament.js
var import_faOrnament = __toESM(require_faOrnament());
var export_faOrnament = import_faOrnament.definition;
export {
  export_faOrnament as faOrnament
};
