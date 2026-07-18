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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmirking.js
var require_faFaceSmirking = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmirking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-smirking";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e397";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm96-48c0 8.8 7.2 16 16 16l32 0c0 17.7 14.3 32 32 32s32-14.3 32-32s-14.3-32-32-32l-64 0c-8.8 0-16 7.2-16 16zM375.8 338.6c1.5-8.7-4.4-17-13.2-18.4s-17 4.4-18.4 13.2l-.2 1.2c-3.2 19.3-19.9 33.4-39.5 33.4L192 368c-8.8 0-16 7.2-16 16s7.2 16 16 16l112.6 0c35.2 0 65.2-25.4 71-60.2l.2-1.2zM288 208c0 8.8 7.2 16 16 16l32 0c0 17.7 14.3 32 32 32s32-14.3 32-32s-14.3-32-32-32l-64 0c-8.8 0-16 7.2-16 16z";
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
    exports.faFaceSmirking = exports.definition;
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

// src/_entries/faFaceSmirking.js
var import_faFaceSmirking = __toESM(require_faFaceSmirking());
var export_faFaceSmirking = import_faFaceSmirking.definition;
export {
  export_faFaceSmirking as faFaceSmirking
};
