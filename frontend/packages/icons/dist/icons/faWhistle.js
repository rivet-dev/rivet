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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWhistle.js
var require_faWhistle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWhistle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "whistle";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f460";
    var svgPathData = "M44.1 188.8C18.5 180.5 0 156.4 0 128C0 92.7 28.7 64 64 64c23.4 0 43.8 12.5 55 31.2C149.2 75.5 185.2 64 224 64c7.8 0 15.4 .5 22.9 1.4c.5 0 1.1 0 1.7 .1c37.8 4.5 73.8 19.4 103.9 43.2l69.8 55.2c5 4 7.2 10.6 5.5 16.8l-6.5 24c-3.2 11.7 1.1 24.2 10.8 31.5c9.6 7.2 22.7 7.9 33 1.7l19.3-11.6c5.7-3.4 12.9-3 18.2 1.2l127 100.4c6.6 5.2 10.5 13.2 10.5 21.6c0 6.9-2.6 13.5-7.2 18.5l-63.3 69.6c-6.1 6.7-14.7 10.5-23.7 10.5l-2 0c-5.2 0-10.2-1.2-14.8-3.6L380.7 367c-34.8 49-92 81-156.7 81C118 448 32 362 32 256c0-23.6 4.3-46.3 12.1-67.2zM80 128a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM224 320a64 64 0 1 0 0-128 64 64 0 1 0 0 128z";
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
    exports.faWhistle = exports.definition;
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

// src/_entries/faWhistle.js
var import_faWhistle = __toESM(require_faWhistle());
var export_faWhistle = import_faWhistle.definition;
export {
  export_faWhistle as faWhistle
};
