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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarWash.js
var require_faCarWash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarWash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-wash";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f5e6";
    var svgPathData = "M64 128c26.5 0 48-16 48-48c0-22.2-23-52-37.2-68.2C69 5.1 59 5.1 53.2 11.8C39 28 16 57.8 16 80c0 26.5 21.5 48 48 48zm160 0c26.5 0 48-16 48-48c0-22.2-23-52-37.2-68.2c-5.8-6.7-15.9-6.7-21.7 0C199 28 176 57.8 176 80c0 26.5 21.5 48 48 48zM432 80c0-22.2-23-52-37.2-68.2c-5.8-6.7-15.9-6.7-21.7 0C359 28 336 57.8 336 80c0 26.5 21.5 48 48 48s48-16 48-48zM153.8 224l140.3 0c6.8 0 12.8 4.3 15.1 10.6L328.3 288l-208.6 0 19.1-53.4c2.3-6.4 8.3-10.6 15.1-10.6zM78.5 213.1L50.2 292.4C30.1 300.9 16 320.8 16 344l0 40 0 64 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 256 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 0-64 0-40c0-23.2-14.1-43.1-34.2-51.6l-28.3-79.3C358.1 181.3 328 160 294.2 160l-140.3 0c-33.8 0-64 21.3-75.3 53.1zM96 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm232 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faCarWash = exports.definition;
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

// src/_entries/faCarWash.js
var import_faCarWash = __toESM(require_faCarWash());
var export_faCarWash = import_faCarWash.definition;
export {
  export_faCarWash as faCarWash
};
