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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrophyStar.js
var require_faTrophyStar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrophyStar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trophy-star";
    var width = 576;
    var height = 512;
    var aliases = ["trophy-alt"];
    var unicode = "f2eb";
    var svgPathData = "M176 0L400 0c26.5 0 48.1 21.8 47.1 48.2c-.2 5.3-.4 10.6-.7 15.8L552 64c13.3 0 24 10.7 24 24c0 92.6-33.5 157-78.5 200.7c-44.3 43.1-98.3 64.8-138.2 75.8c-23.4 6.5-39.4 26-39.4 45.6c0 20.9 17 37.9 37.9 37.9l26.1 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-192 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l26.1 0c20.9 0 37.9-17 37.9-37.9c0-19.6-15.9-39.2-39.4-45.6c-39.9-11-93.9-32.7-138.1-75.8C33.5 245 0 180.6 0 88C0 74.7 10.7 64 24 64l105.6 0c-.3-5.2-.5-10.4-.7-15.8C127.9 21.8 149.5 0 176 0zM133.2 112l-84.4 0c5.1 66.3 31.1 111.2 63 142.3c22.4 21.8 48.3 37.3 73.2 48.3c-22.7-40.3-42.8-100.5-51.9-190.6zM390.8 302.6c24.9-11 50.8-26.5 73.2-48.3c32-31.1 58-76 63-142.3l-84.4 0c-9.1 90.1-29.2 150.3-51.9 190.6zM295.2 86.5c-2.9-5.9-11.4-5.9-14.3 0l-19.2 38.9c-1.2 2.4-3.4 4-6 4.4L212.7 136c-6.6 1-9.2 9-4.4 13.6l31 30.2c1.9 1.8 2.7 4.5 2.3 7.1l-7.3 42.7c-1.1 6.5 5.7 11.5 11.6 8.4L284.3 218c2.3-1.2 5.1-1.2 7.4 0l38.4 20.2c5.9 3.1 12.7-1.9 11.6-8.4L334.4 187c-.4-2.6 .4-5.2 2.3-7.1l31-30.2c4.7-4.6 2.1-12.7-4.4-13.6l-42.9-6.2c-2.6-.4-4.9-2-6-4.4L295.2 86.5z";
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
    exports.faTrophyStar = exports.definition;
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

// src/_entries/faTrophyAlt.js
var import_faTrophyStar = __toESM(require_faTrophyStar());
var export_faTrophyAlt = import_faTrophyStar.definition;
export {
  export_faTrophyAlt as faTrophyAlt
};
