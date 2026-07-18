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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandLove.js
var require_faHandLove = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandLove.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-love";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1a5";
    var svgPathData = "M160 0c17.7 0 32 14.3 32 32l0 159.9 0 .1 0 96c0 35.3 28.7 64 64 64c19.1 0 36.3-8.4 48-21.7c11.7 13.3 28.9 21.7 48 21.7c35.3 0 64-28.7 64-64l0-63.9c0 0 0 0 0-.1l0-128c0-17.7 14.3-32 32-32s32 14.3 32 32l0 240c0 97.2-78.8 176-176 176l-19.2 0c-59.6 0-116.9-22.9-160-64L12.4 341c-16-15.2-16.6-40.6-1.4-56.6s40.6-16.6 56.6-1.4l60.5 57.6c0-1.5-.1-3.1-.1-4.6l0-304c0-17.7 14.3-32 32-32zm64 192c0-17.7 14.3-32 32-32s32 14.3 32 32l0 96c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-96zm128 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32z";
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
    exports.faHandLove = exports.definition;
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

// src/_entries/faHandLove.js
var import_faHandLove = __toESM(require_faHandLove());
var export_faHandLove = import_faHandLove.definition;
export {
  export_faHandLove as faHandLove
};
