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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faIceSkate.js
var require_faIceSkate = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faIceSkate.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ice-skate";
    var width = 576;
    var height = 512;
    var aliases = [9976];
    var unicode = "f7ac";
    var svgPathData = "M512 336c0 26.5-21.5 48-48 48L80 384c-26.5 0-48-21.5-48-48l0-203.8c0-21.4 14.2-40.3 34.8-46.2L256 32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 64-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l48 0 0 32-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l48 0 119.3 29.8C482 232.5 512 270.9 512 315l0 21zm40 72c13.3 0 24 10.7 24 24l0 8c0 39.8-32.2 72-72 72l-96 0-288 0-96 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l72 0 0-48 48 0 0 48 240 0 0-48 48 0 0 48 72 0c13.3 0 24-10.7 24-24l0-8c0-13.3 10.7-24 24-24z";
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
    exports.faIceSkate = exports.definition;
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

// src/_entries/faIceSkate.js
var import_faIceSkate = __toESM(require_faIceSkate());
var export_faIceSkate = import_faIceSkate.definition;
export {
  export_faIceSkate as faIceSkate
};
