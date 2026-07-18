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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFolderTree.js
var require_faFolderTree = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFolderTree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "folder-tree";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f802";
    var svgPathData = "M48 24C48 10.7 37.3 0 24 0S0 10.7 0 24L0 392c0 30.9 25.1 56 56 56l184 0 0-48-184 0c-4.4 0-8-3.6-8-8l0-232 192 0 0-48-192 0 0-88zM336 224l192 0c26.5 0 48-21.5 48-48l0-96c0-26.5-21.5-48-48-48l-82.7 0c-8.5 0-16.6-3.4-22.6-9.4l-8.6-8.6c-9-9-21.2-14.1-33.9-14.1L336 0c-26.5 0-48 21.5-48 48l0 128c0 26.5 21.5 48 48 48zm0 288l192 0c26.5 0 48-21.5 48-48l0-96c0-26.5-21.5-48-48-48l-82.7 0c-8.5 0-16.6-3.4-22.6-9.4l-8.6-8.6c-9-9-21.2-14.1-33.9-14.1L336 288c-26.5 0-48 21.5-48 48l0 128c0 26.5 21.5 48 48 48z";
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
    exports.faFolderTree = exports.definition;
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

// src/_entries/faFolderTree.js
var import_faFolderTree = __toESM(require_faFolderTree());
var export_faFolderTree = import_faFolderTree.definition;
export {
  export_faFolderTree as faFolderTree
};
