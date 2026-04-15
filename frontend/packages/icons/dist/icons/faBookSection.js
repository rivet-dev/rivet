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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookSection.js
var require_faBookSection = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookSection.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-section";
    var width = 448;
    var height = 512;
    var aliases = ["book-law"];
    var unicode = "e0c1";
    var svgPathData = "M0 96C0 43 43 0 96 0L384 0l32 0c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32l0 64c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0L96 512c-53 0-96-43-96-96L0 96zM64 416c0 17.7 14.3 32 32 32l256 0 0-64L96 384c-17.7 0-32 14.3-32 32zM228.5 241.3l7.5 2c37.9 9.8 46.1 15 43.6 27.7c-2.5 13.2-23.7 19.6-51.7 15.7c-10.2-1.4-22.4-5.4-33.1-8.9l-5.9-1.9c-8.4-2.7-17.4 1.9-20.1 10.4c-2.7 8.4 2 17.4 10.4 20.1l5.8 1.9c12 3.9 25.7 8.3 38.6 10.1c7 1 13.8 1.5 20.2 1.5c36.5 0 62.2-15.8 67.3-42.8c4.1-21.2-4-34.9-16.7-44.4c8.7-7.1 14.5-16.4 16.7-27.6c8.6-45.1-36.9-56.9-67-64.7c0 0 0 0 0 0l-7.3-1.9c-36.5-9.7-38-16.5-36.2-25.8c2.5-13.2 23.8-19.7 51.6-15.7c6.6 .9 14.6 2.9 25.2 6.2c8.3 2.6 17.4-2.1 20-10.5c2.6-8.4-2.1-17.4-10.5-20c-12.2-3.8-21.9-6.2-30.2-7.3c-47.1-6.7-81.5 9.6-87.5 41.3c-4.1 21.3 4.4 35.1 16.7 44.5c-8.7 7.1-14.5 16.4-16.7 27.5c-8.5 44.6 37.4 56.8 59.5 62.7zm-2.9-72.8c.9 .2 10.3 2.7 10.3 2.7c37.9 9.8 46.1 15 43.6 27.7c-2.2 11.3-16.5 14.9-25.2 16.1c-3.6-1-17.7-4.7-17.7-4.7c-36.5-9.7-38-16.5-36.2-25.8c2.2-11.3 16.5-14.9 25.2-16.1z";
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
    exports.faBookSection = exports.definition;
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

// src/_entries/faBookSection.js
var import_faBookSection = __toESM(require_faBookSection());
var export_faBookSection = import_faBookSection.definition;
export {
  export_faBookSection as faBookSection
};
