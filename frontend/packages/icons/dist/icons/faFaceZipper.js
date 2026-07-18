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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceZipper.js
var require_faFaceZipper = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceZipper.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-zipper";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3a5";
    var svgPathData = "M492.4 354.4C505 324.1 512 290.9 512 256C512 114.6 397.4 0 256 0S0 114.6 0 256S114.6 512 256 512c50.3 0 97.2-14.5 136.7-39.5c-1.1-1.8-2.2-3.7-3.1-5.7l-29.1-58.1c-9.2-18.5-5.6-40.8 9-55.4l15.7-15.7c14.6-14.6 36.9-18.2 55.4-9l51.8 25.9zM176.4 176a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm128 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM144 336l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16s16 7.2 16 16zm48-16c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zm80 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16s16 7.2 16 16zm48-16c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zm98.2 132.4c14.7 29.3 53.7 35.6 76.9 12.5l1.8-1.8c23.2-23.2 16.8-62.2-12.5-76.9l-58.1-29.1c-6.2-3.1-13.6-1.9-18.5 3l-15.7 15.7c-4.9 4.9-6.1 12.3-3 18.5l29.1 58.1zm62.3-36.9c4.7 4.7 4.7 12.3 0 17l-16 16c-4.7 4.7-12.3 4.7-17 0s-4.7-12.3 0-17l16-16c4.7-4.7 12.3-4.7 17 0z";
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
    exports.faFaceZipper = exports.definition;
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

// src/_entries/faFaceZipper.js
var import_faFaceZipper = __toESM(require_faFaceZipper());
var export_faFaceZipper = import_faFaceZipper.definition;
export {
  export_faFaceZipper as faFaceZipper
};
