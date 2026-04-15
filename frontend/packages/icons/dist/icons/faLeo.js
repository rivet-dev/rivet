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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLeo.js
var require_faLeo = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLeo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "leo";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e84a";
    var svgPathData = "M260 0c72.9 0 132 59.1 132 132l0 5.4-.1 1.4-23.8 270c.4 21.7 18.2 39.2 40 39.2 22.1 0 40-17.9 40-40l0-40c0-17.7 14.3-32 32-32s32 14.3 32 32l0 40c0 57.4-46.6 104-104 104S304 465.4 304 408l0-1.4 .1-1.4 23.9-270.6 0-2.6c0-37.6-30.4-68-68-68s-68 30.4-68 68l0 4c0 3.7 .3 7.3 .8 11l29.7 193.4c.9 6.1 1.4 12.2 1.4 18.4l0 9.2c0 61.9-50.1 112-112 112S0 429.9 0 368 50.1 256 112 256c11.7 0 23 1.8 33.7 5.1L129.6 156.7c-1.1-6.8-1.6-13.8-1.6-20.7l0-4C128 59.1 187.1 0 260 0zM112 320a48 48 0 1 0 0 96 48 48 0 1 0 0-96z";
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
    exports.faLeo = exports.definition;
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

// src/_entries/faLeo.js
var import_faLeo = __toESM(require_faLeo());
var export_faLeo = import_faLeo.definition;
export {
  export_faLeo as faLeo
};
