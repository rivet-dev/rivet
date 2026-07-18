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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkSimpleSlash.js
var require_faLinkSimpleSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkSimpleSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "link-simple-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1ce";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L532.4 392c45.4-28.2 75.6-78.6 75.6-136c0-88.4-71.6-160-160-160l-64 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l64 0c53 0 96 43 96 96c0 43.3-28.6 79.8-67.9 91.8L399.7 288l16.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-97.9 0-81.7-64 19.6 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0c-11.3 0-22.3 1.2-32.9 3.4L38.8 5.1zM121 191.4L70.7 151.7C46.6 179.7 32 216.1 32 256c0 88.4 71.6 160 160 160l64 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0c-53 0-96-43-96-96c0-24.9 9.5-47.6 25-64.6z";
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
    exports.faLinkSimpleSlash = exports.definition;
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

// src/_entries/faLinkSimpleSlash.js
var import_faLinkSimpleSlash = __toESM(require_faLinkSimpleSlash());
var export_faLinkSimpleSlash = import_faLinkSimpleSlash.definition;
export {
  export_faLinkSimpleSlash as faLinkSimpleSlash
};
