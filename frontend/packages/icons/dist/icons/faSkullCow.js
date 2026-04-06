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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSkullCow.js
var require_faSkullCow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSkullCow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "skull-cow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8de";
    var svgPathData = "M63.2 21.1c2.5-7.4-.7-15.4-7.5-19.1S40.3 0 35.5 6L28 15.3C9.9 38 0 66.4 0 95.5C0 166.1 57.2 224 128 224l0-16 0-16 0-32 0-16 0-16-23 0c-28.7 0-52.7-21.7-55.6-50.3c-.8-7.8 .1-15.8 2.6-23.2L63.2 21.1zM240 96c-44.2 0-80 35.8-80 80l0 112c0 37 25.2 68.2 59.3 77.3l27.6 110.3c5.3 21.4 24.5 36.4 46.6 36.4l53 0c22 0 41.2-15 46.6-36.4l27.6-110.3C454.8 356.2 480 325 480 288l0-112c0-44.2-35.8-80-80-80L240 96zM224 256a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm160-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM584.4 1.9c-6.8 3.7-10 11.8-7.5 19.1L588 54.5c2.5 7.5 3.4 15.4 2.6 23.2C587.7 106.3 563.7 128 535 128l-23 0 0 16 0 16 0 32 0 16 0 16c70.8 0 128-57.9 128-128.5c0-29-9.9-57.4-28-80.1L604.5 6c-4.8-6.1-13.3-7.8-20.1-4.1z";
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
    exports.faSkullCow = exports.definition;
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

// src/_entries/faSkullCow.js
var import_faSkullCow = __toESM(require_faSkullCow());
var export_faSkullCow = import_faSkullCow.definition;
export {
  export_faSkullCow as faSkullCow
};
