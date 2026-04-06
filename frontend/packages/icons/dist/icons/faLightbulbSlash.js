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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbSlash.js
var require_faLightbulbSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lightbulb-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f673";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-195-152.8c4.4-6.2 8.9-12.4 13.4-18.6c0 0 0 0 0 0s0 0 0 0s0 0 0 0c5.2-7.1 10.4-14.2 15.4-21.4c19.8-28.5 31.4-63 31.4-100.3C496 78.8 417.2 0 320 0C249.7 0 189.1 41.2 160.8 100.8L38.8 5.1zM213.3 141.9C227.7 96.7 270 64 320 64c8.8 0 16 7.2 16 16s-7.2 16-16 16c-40 0-73.1 29.3-79 67.6l-27.7-21.7zm-60.9 87.9c5.4 16.8 13.2 32.4 23 46.6c5 7.2 10.2 14.3 15.4 21.4c0 0 0 0 0 0c19.8 27.1 39.7 54.4 49.2 86.2l108.1 0L152.4 229.8zM240 416l0 16c0 44.2 35.8 80 80 80s80-35.8 80-80l0-7.1L388.7 416 240 416z";
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
    exports.faLightbulbSlash = exports.definition;
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

// src/_entries/faLightbulbSlash.js
var import_faLightbulbSlash = __toESM(require_faLightbulbSlash());
var export_faLightbulbSlash = import_faLightbulbSlash.definition;
export {
  export_faLightbulbSlash as faLightbulbSlash
};
