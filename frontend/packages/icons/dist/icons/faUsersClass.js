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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScreenUsers.js
var require_faScreenUsers = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScreenUsers.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "screen-users";
    var width = 640;
    var height = 512;
    var aliases = ["users-class"];
    var unicode = "f63d";
    var svgPathData = "M96 0C60.7 0 32 28.7 32 64l0 184.4C49 233.2 71.4 224 96 224L96 64l448 0 0 160c24.6 0 47 9.2 64 24.4L608 64c0-35.3-28.7-64-64-64L96 0zm64 320A64 64 0 1 0 32 320a64 64 0 1 0 128 0zM0 480c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-64 0c-35.3 0-64 28.7-64 64zM384 320a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zM224 480c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-64 0c-35.3 0-64 28.7-64 64zm320-96a64 64 0 1 0 0-128 64 64 0 1 0 0 128zm-32 32c-35.3 0-64 28.7-64 64c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-64 0z";
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
    exports.faScreenUsers = exports.definition;
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

// src/_entries/faUsersClass.js
var import_faScreenUsers = __toESM(require_faScreenUsers());
var export_faUsersClass = import_faScreenUsers.definition;
export {
  export_faUsersClass as faUsersClass
};
