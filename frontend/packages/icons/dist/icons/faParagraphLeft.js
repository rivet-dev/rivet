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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faParagraphLeft.js
var require_faParagraphLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faParagraphLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paragraph-left";
    var width = 384;
    var height = 512;
    var aliases = ["paragraph-rtl"];
    var unicode = "f878";
    var svgPathData = "M320 64c17.7 0 32-14.3 32-32s-14.3-32-32-32L160 0 144 0C82.1 0 32 50.1 32 112s50.1 112 112 112l16 0 0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-224 32 0 0 224c0 17.7 14.3 32 32 32s32-14.3 32-32l0-224zM128 384l0-46.4c0-9.7-7.9-17.6-17.6-17.6c-4.2 0-8.2 1.5-11.3 4.2l-93 78.7C2.2 406.1 0 410.9 0 416s2.2 9.9 6.1 13.2l93 78.7c3.2 2.7 7.2 4.2 11.3 4.2c9.7 0 17.6-7.9 17.6-17.6l0-46.4 224 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-224 0z";
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
    exports.faParagraphLeft = exports.definition;
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

// src/_entries/faParagraphLeft.js
var import_faParagraphLeft = __toESM(require_faParagraphLeft());
var export_faParagraphLeft = import_faParagraphLeft.definition;
export {
  export_faParagraphLeft as faParagraphLeft
};
