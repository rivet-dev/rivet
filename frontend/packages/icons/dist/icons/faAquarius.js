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

// src/node_modules/@fortawesome/free-solid-svg-icons/faAquarius.js
var require_faAquarius = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faAquarius.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "aquarius";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e845";
    var svgPathData = "M401.7 291.4c9-4.5 19.6-4.5 28.6 0l128 64c15.8 7.9 22.2 27.1 14.3 42.9s-27.1 22.2-42.9 14.3L416 355.8 302.3 412.6c-9 4.5-19.6 4.5-28.6 0L160 355.8 46.3 412.6c-15.8 7.9-35 1.5-42.9-14.3s-1.5-35 14.3-42.9l128-64c9-4.5 19.6-4.5 28.6 0L288 348.2 401.7 291.4zm3.4-193.5c8.2-3 17.3-2.5 25.2 1.5l128 64c15.8 7.9 22.2 27.1 14.3 42.9s-27.1 22.2-42.9 14.3L416 163.8 302.3 220.6c-9 4.5-19.6 4.5-28.6 0L160 163.8 46.3 220.6c-15.8 7.9-35 1.5-42.9-14.3s-1.5-35 14.3-42.9l128-64 3.4-1.5c8.2-3 17.3-2.5 25.2 1.5l113.7 56.8 113.7-56.8 3.4-1.5z";
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
    exports.faAquarius = exports.definition;
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

// src/_entries/faAquarius.js
var import_faAquarius = __toESM(require_faAquarius());
var export_faAquarius = import_faAquarius.definition;
export {
  export_faAquarius as faAquarius
};
