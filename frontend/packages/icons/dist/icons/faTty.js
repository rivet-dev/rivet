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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTty.js
var require_faTty = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tty";
    var width = 512;
    var height = 512;
    var aliases = ["teletype"];
    var unicode = "f1e4";
    var svgPathData = "M450.2 266.8c15.8 6.5 34.1 .3 42.6-14.6l2.4-4.2c27.9-48.9 23.6-118.8-31.3-154.5-126-82-289.6-82-415.6 0-54.9 35.7-59.3 105.7-31.3 154.5l2.4 4.2c8.5 14.9 26.7 21.1 42.6 14.6l81.9-33.7c13.9-5.7 22.4-19.9 20.9-34.9l-5.1-51c62.5-21 130.8-19.9 192.6 3.3l-4.8 47.7c-1.5 15 7 29.2 20.9 34.9l81.9 33.7zM32 352a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zm96 0a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM64 416a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm352 32a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM256 320a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm64 32a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zm128-32a32 32 0 1 0 0 64 32 32 0 1 0 0-64zM128 448c0 17.7 14.3 32 32 32l192 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-192 0c-17.7 0-32 14.3-32 32z";
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
    exports.faTty = exports.definition;
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

// src/_entries/faTty.js
var import_faTty = __toESM(require_faTty());
var export_faTty = import_faTty.definition;
export {
  export_faTty as faTty
};
