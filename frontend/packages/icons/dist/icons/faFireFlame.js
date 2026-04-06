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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFireFlame.js
var require_faFireFlame = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFireFlame.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fire-flame";
    var width = 384;
    var height = 512;
    var aliases = ["flame"];
    var unicode = "f6df";
    var svgPathData = "M372.5 256.5l-.7-1.9C337.8 160.8 282 76.5 209.1 8.5l-3.3-3C202.1 2 197.1 0 192 0s-10.1 2-13.8 5.5l-3.3 3C102 76.5 46.2 160.8 12.2 254.6l-.7 1.9C3.9 277.3 0 299.4 0 321.6C0 426.7 86.8 512 192 512s192-85.3 192-190.4c0-22.2-3.9-44.2-11.5-65.1zM188.8 148.3c2-2.7 5.2-4.3 8.5-4.3c5.9 0 10.7 4.8 10.7 10.7l0 11.4c0 8.9 3.6 17.4 9.9 23.6l51.5 50.7C291.5 262.2 304 292 304 323c0 60.2-48.8 109-109 109l-3 0c-61.9 0-112-50.1-112-112l0-8.2c0-21.2 7.8-41.6 21.8-57.4l6.9-7.8c2.1-2.4 5.1-3.7 8.3-3.7c6.1 0 11 4.9 11 11l0 44c0 24.3 19.8 44 44.1 44c24.2 0 43.9-19.6 43.9-43.8c0-11.6-4.6-22.8-12.8-31l-13.2-13.2c-14-14-21.9-33.1-21.9-53c0-16.2 5.3-32 15-44.9l5.8-7.8z";
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
    exports.faFireFlame = exports.definition;
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

// src/_entries/faFireFlame.js
var import_faFireFlame = __toESM(require_faFireFlame());
var export_faFireFlame = import_faFireFlame.definition;
export {
  export_faFireFlame as faFireFlame
};
