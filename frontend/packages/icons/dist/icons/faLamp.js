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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLamp.js
var require_faLamp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLamp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lamp";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f4ca";
    var svgPathData = "M82.5 19.7C87.4 7.8 99.1 0 112 0L336 0c12.9 0 24.6 7.8 29.5 19.7l80 192c4.1 9.9 3 21.2-2.9 30.1s-15.9 14.2-26.6 14.2L32 256c-10.7 0-20.7-5.3-26.6-14.2s-7-20.2-2.9-30.1l80-192zM160 288l128 0 0 32s64 50.5 64 112c0 24.4-8.3 44.6-16.4 58.7c-8.5 14.7-25.2 21.3-42.1 21.3l-139 0c-17 0-33.6-6.6-42.1-21.3C104.3 476.6 96 456.4 96 432c0-61.5 64-112 64-112l0-32z";
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
    exports.faLamp = exports.definition;
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

// src/_entries/faLamp.js
var import_faLamp = __toESM(require_faLamp());
var export_faLamp = import_faLamp.definition;
export {
  export_faLamp as faLamp
};
