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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneProp.js
var require_faPlaneProp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneProp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-prop";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e22b";
    var svgPathData = "M240 384l-8.3-66.4-110-11L113.8 370c-1 8-7.8 14-15.9 14l-35.8 0c-8.1 0-14.9-6-15.9-14L36 287.8c-1.3 .2-2.6 .2-4 .2c-17.7 0-32-14.3-32-32s14.3-32 32-32c1.3 0 2.7 .1 4 .2L46.2 142c1-8 7.8-14 15.9-14l35.8 0c8.1 0 14.9 6 15.9 14l7.9 63.4 110-11L240 128l-16 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l24 0 6.2-50c1-8 7.8-14 15.9-14l35.8 0c8.1 0 14.9 6 15.9 14L328 64l40 0 0-16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 32c8.8 0 16 7.2 16 16s-7.2 16-16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-16-32 0 8 64 138.3 0c34.2 0 93.7 29 93.7 64c0 36-59.5 64-93.7 64L344 320l-8 64 32 0 0-16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 32c8.8 0 16 7.2 16 16s-7.2 16-16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-16-40 0-6.2 50c-1 8-7.8 14-15.9 14l-35.8 0c-8.1 0-14.9-6-15.9-14L248 448l-24 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l16 0z";
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
    exports.faPlaneProp = exports.definition;
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

// src/_entries/faPlaneProp.js
var import_faPlaneProp = __toESM(require_faPlaneProp());
var export_faPlaneProp = import_faPlaneProp.definition;
export {
  export_faPlaneProp as faPlaneProp
};
