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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHelmetUn.js
var require_faHelmetUn = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHelmetUn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "helmet-un";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e503";
    var svgPathData = "M479.5 224C471.2 98.9 367.2 0 240 0 107.5 0 0 107.5 0 240l0 56.3C0 344.8 39.2 384 87.7 384l127.3 0 128.6 121.4c4.5 4.2 10.4 6.6 16.5 6.6l96 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-86.5 0-1.5-1.5 0-174.5 112 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-.5 0zM320 417.2l-78-73.7 32.4-55.5 45.6 0 0 129.2zM285.3 103.1l34.7 52 0-43.2c0-8.8 7.2-16 16-16s16 7.2 16 16l0 96c0 7.1-4.6 13.3-11.4 15.3s-14-.6-17.9-6.4l-34.7-52 0 43.2c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-96c0-7.1 4.6-13.3 11.4-15.3s14 .6 17.9 6.4zM160 112l0 64c0 8.8 7.2 16 16 16s16-7.2 16-16l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16l0 64c0 26.5-21.5 48-48 48s-48-21.5-48-48l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16z";
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
    exports.faHelmetUn = exports.definition;
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

// src/_entries/faHelmetUn.js
var import_faHelmetUn = __toESM(require_faHelmetUn());
var export_faHelmetUn = import_faHelmetUn.definition;
export {
  export_faHelmetUn as faHelmetUn
};
