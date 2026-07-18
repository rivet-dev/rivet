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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUsersRays.js
var require_faUsersRays = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUsersRays.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "users-rays";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e593";
    var svgPathData = "M31-33C21.7-23.6 21.7-8.4 31 1L87 57c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L65-33c-9.4-9.4-24.6-9.4-33.9 0zM609-33c-9.4-9.4-24.6-9.4-33.9 0L519 23c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0L609 1c9.4-9.4 9.4-24.6 0-33.9zM65 545l56-56c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L31 511c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0zm544 0c9.4-9.4 9.4-24.6 0-33.9l-56-56c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l56 56c9.4 9.4 24.6 9.4 33.9 0zM320 208a64 64 0 1 0 0-128 64 64 0 1 0 0 128zm0 32c-61.9 0-112 50.1-112 112l0 8c0 13.3 10.7 24 24 24l176 0c13.3 0 24-10.7 24-24l0-8c0-61.9-50.1-112-112-112zm216-80a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zM451.2 260.4C469.4 286.3 480 317.9 480 352l0 8c0 8.4-1.4 16.5-4.1 24l78.8 0c11.8 0 21.3-9.6 21.3-21.3l0-10.7c0-53-43-96-96-96-10 0-19.7 1.5-28.8 4.4zm-262.4 0c-9.1-2.9-18.8-4.4-28.8-4.4-53 0-96 43-96 96l0 10.7c0 11.8 9.6 21.3 21.3 21.3l78.8 0c-2.7-7.5-4.1-15.6-4.1-24l0-8c0-34.1 10.6-65.7 28.8-91.6zM216 160a56 56 0 1 0 -112 0 56 56 0 1 0 112 0z";
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
    exports.faUsersRays = exports.definition;
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

// src/_entries/faUsersRays.js
var import_faUsersRays = __toESM(require_faUsersRays());
var export_faUsersRays = import_faUsersRays.definition;
export {
  export_faUsersRays as faUsersRays
};
