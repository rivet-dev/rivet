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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarker.js
var require_faMarker = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarker.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "marker";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5a1";
    var svgPathData = "M408.8 0c-27.4 0-53.6 10.9-73 30.2L318.1 48 305 34.9c-28.1-28.1-73.7-28.1-101.8 0L103 135c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0L237.1 68.9c9.4-9.4 24.6-9.4 33.9 0L284.1 81.9 184 182.1 329.9 328 481.8 176.2c19.4-19.4 30.2-45.6 30.2-73 0-57-46.2-103.2-103.2-103.2zM102.4 263.7c-49.9 49.9-83.3 114-95.5 183.5L.4 483.8C-1 491.6 1.5 499.4 7 505s13.4 8 21.1 6.7l36.7-6.5c69.5-12.3 133.6-45.6 183.5-95.5L296 361.9 150.1 216 102.4 263.7z";
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
    exports.faMarker = exports.definition;
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

// src/_entries/faMarker.js
var import_faMarker = __toESM(require_faMarker());
var export_faMarker = import_faMarker.definition;
export {
  export_faMarker as faMarker
};
