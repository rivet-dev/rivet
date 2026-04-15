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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUtensilsSlash.js
var require_faUtensilsSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUtensilsSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "utensils-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e464";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-86.8-68 0-49.1 0-112 0-208c0-17.7-14.3-32-32-32C496 0 384 32 384 176l0 99.7-77.6-60.8c8.6-13.6 13.6-29.7 13.6-47c0-6.4-.7-12.8-2.1-19.1L287.6 12.5c-1.8-8-9.3-13.3-17.4-12.4S256 7.8 256 16l0 134.2c0 5.4-4.4 9.8-9.8 9.8c-5.1 0-9.3-3.9-9.8-9L223.9 14.6C223.2 6.3 216.3 0 208 0s-15.2 6.3-15.9 14.6l-9.4 103.3L160 100.1 160 16c0-8.2-6.1-15-14.3-15.9s-15.6 4.5-17.4 12.4L116.5 66 38.8 5.1zM176 255.6L176 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-194.9L96.1 171.7c1.9 44.2 36.3 80 79.9 83.9z";
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
    exports.faUtensilsSlash = exports.definition;
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

// src/_entries/faUtensilsSlash.js
var import_faUtensilsSlash = __toESM(require_faUtensilsSlash());
var export_faUtensilsSlash = import_faUtensilsSlash.definition;
export {
  export_faUtensilsSlash as faUtensilsSlash
};
