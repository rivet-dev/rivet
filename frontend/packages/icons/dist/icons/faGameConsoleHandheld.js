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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGameConsoleHandheld.js
var require_faGameConsoleHandheld = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGameConsoleHandheld.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "game-console-handheld";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f8bb";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L320 0c35.3 0 64 28.7 64 64l0 352c0 53-43 96-96 96L96 512c-53 0-96-43-96-96L0 64zM96 80l0 112c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32l0-112c0-8.8-7.2-16-16-16L112 64c-8.8 0-16 7.2-16 16zM232 384a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm88-72a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM96 304l0 16-16 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0 0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16 16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-16 0 0-16c0-8.8-7.2-16-16-16s-16 7.2-16 16z";
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
    exports.faGameConsoleHandheld = exports.definition;
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

// src/_entries/faGameConsoleHandheld.js
var import_faGameConsoleHandheld = __toESM(require_faGameConsoleHandheld());
var export_faGameConsoleHandheld = import_faGameConsoleHandheld.definition;
export {
  export_faGameConsoleHandheld as faGameConsoleHandheld
};
