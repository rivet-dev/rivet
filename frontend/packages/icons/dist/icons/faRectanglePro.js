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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRectanglePro.js
var require_faRectanglePro = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRectanglePro.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rectangle-pro";
    var width = 640;
    var height = 512;
    var aliases = ["pro"];
    var unicode = "e235";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l512 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm40 128l56 0c35.3 0 64 28.7 64 64s-28.7 64-64 64l-32 0 0 40c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-64 0-80c0-13.3 10.7-24 24-24zm56 80c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0 32 32 0zm80-56c0-13.3 10.7-24 24-24l56 0c35.3 0 64 28.7 64 64c0 21.9-11 41.2-27.7 52.7l24.1 38.5c7 11.2 3.6 26-7.6 33.1s-26 3.6-33.1-7.6l-33-52.7L288 288l0 40c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-64 0-80zm48 56l32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0 32zm200-80c48.6 0 88 39.4 88 88l0 16c0 48.6-39.4 88-88 88s-88-39.4-88-88l0-16c0-48.6 39.4-88 88-88zm-40 88l0 16c0 22.1 17.9 40 40 40s40-17.9 40-40l0-16c0-22.1-17.9-40-40-40s-40 17.9-40 40z";
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
    exports.faRectanglePro = exports.definition;
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

// src/_entries/faRectanglePro.js
var import_faRectanglePro = __toESM(require_faRectanglePro());
var export_faRectanglePro = import_faRectanglePro.definition;
export {
  export_faRectanglePro as faRectanglePro
};
