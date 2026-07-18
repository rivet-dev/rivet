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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSunDust.js
var require_faSunDust = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSunDust.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sun-dust";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f764";
    var svgPathData = "M371.1 13.1L391 121l-44.5 44.5C323.3 142.3 291.3 128 256 128c-70.7 0-128 57.3-128 128c0 35.3 14.3 67.3 37.5 90.5L121 391 13.1 371.1c-5.3-1-9.8-4.6-11.9-9.6s-1.5-10.7 1.6-15.2L65.1 256 2.8 165.7c-3.1-4.5-3.7-10.2-1.6-15.2s6.6-8.6 11.9-9.6L121 121 140.9 13.1c1-5.3 4.6-9.8 9.6-11.9s10.7-1.5 15.2 1.6L256 65.1 346.3 2.8c4.5-3.1 10.2-3.7 15.2-1.6s8.6 6.6 9.6 11.9zM256 160c26.5 0 50.5 10.7 67.9 28.1L188.1 323.9C170.7 306.5 160 282.5 160 256c0-53 43-96 96-96zm192 0a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm-80 80a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm112 48a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM240 368a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm-80 80a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm128 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm192-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM368 400a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM320 288a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faSunDust = exports.definition;
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

// src/_entries/faSunDust.js
var import_faSunDust = __toESM(require_faSunDust());
var export_faSunDust = import_faSunDust.definition;
export {
  export_faSunDust as faSunDust
};
